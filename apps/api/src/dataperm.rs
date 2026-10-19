//! Data-access permission evaluation (role rules → per-request decision set).
//!
//! The PDP (`security-pdp`) evaluates coarse system capabilities. Data-access
//! rules — per object type, field, entity instance, relation type/instance and
//! URL — are evaluated here against `sec.policy_rule` rows of the principal's
//! effective groups (direct memberships + ancestor groups via `parent_id`).
//!
//! Action grammar stored in `sec.policy_rule.action`:
//!   data.<scope>.<op>   where scope ∈ {type, field, entity, relation,
//!                        relation_instance, all}  and op ∈ {read, create,
//!                        update, delete}  ('*' is the wildcard op)
//!   system.url          resource.path holds the URL prefix
//!   data.*              legacy wildcard — allows every data operation
//!
//! Resource selector (`sec.policy_rule.resource` jsonb):
//!   {type: "<alias|type_uuid>"}            for type scope ('*'/absent = all)
//!   {type: ..., field: "<alias>"}          for field scope
//!   {entity: "<public_id>"}                for entity instance scope
//!   {predicate: "<alias>", direction: "out|in|both"} for relation scope
//!   {relation: "<relation_id>"}            for relation instance scope
//!   {path: "/prefix"}                      for system.url
//!
//! Defaults (per requirements): type/field/relation scopes default DENY;
//! entity/relation-instance scopes default ALLOW (rules only deny); URL
//! access is open unless a blanket deny exists, in which case a matching
//! allow rule is required. Deny always overrides allow.
//!
//! Special roles (group alias): 'superadmin' → everything allowed;
//! 'superuser' → everything allowed except administrative capabilities and
//! admin URLs.

use std::collections::HashMap;

use enginer_db_runtime::DbError;
use serde_json::Value;
use sqlx::{Postgres, Row, Transaction};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Op {
    Read,
    Create,
    Update,
    Delete,
}

impl Op {
    fn index(self) -> usize {
        match self {
            Op::Read => 0,
            Op::Create => 1,
            Op::Update => 2,
            Op::Delete => 3,
        }
    }
}

/// Capabilities treated as administrative — SUPERUSER never gets these.
pub const ADMIN_CAPS: &[&str] = &[
    "schema.draft",
    "schema.publish",
    "security.manage",
    "security.simulate",
    "users.manage",
    "user.read",
    "user.create",
    "user.edit",
    "user.delete",
    "connector.manage",
    "admin.pages.access",
];

/// URL prefixes reserved for administrative API surface.
pub const ADMIN_URL_PREFIXES: &[&str] = &[
    "/admin/dump",
    "/permissions",
    "/users",
    "/audit/retention",
    "/audit/retire",
    "/audit/checkpoint",
    "/schemas/drafts",
    "/schemas/publish",
];

#[derive(Debug, Default)]
struct PermSet {
    allow: [bool; 4],
    deny: [bool; 4],
}

impl PermSet {
    fn apply(&mut self, op: Op, effect: &str) {
        let i = op.index();
        if effect == "deny" {
            self.deny[i] = true;
        } else {
            self.allow[i] = true;
        }
    }
    fn apply_all(&mut self, effect: &str) {
        for op in [Op::Read, Op::Create, Op::Update, Op::Delete] {
            self.apply(op, effect);
        }
    }
    fn granted(&self, op: Op) -> bool {
        !self.deny[op.index()] && self.allow[op.index()]
    }
    fn denied(&self, op: Op) -> bool {
        self.deny[op.index()]
    }
}

/// Compiled data-permission context for one principal/tenant.
#[derive(Debug, Default)]
pub struct DataPerm {
    pub superadmin: bool,
    pub superuser: bool,
    /// `data.*` (or `data.all.*`) allow present.
    wildcard_allow: bool,
    type_rules: HashMap<String, PermSet>,
    field_rules: HashMap<(String, String), PermSet>,
    entity_rules: HashMap<String, PermSet>,
    /// Relation-type rules keyed by (predicate, direction): direction is
    /// `""` (both), `"out"` or `"in"`.
    rel_rules: HashMap<(String, String), PermSet>,
    rel_inst_rules: HashMap<String, PermSet>,
    /// `data.all.<op>` — grants/denies one op across every scope.
    all_rules: PermSet,
    /// Field masks keyed by (type, field) — applied when read is allowed but
    /// the field must be redacted (policy_rule.field_mask).
    field_masks: HashMap<(String, String), String>,
    /// (path prefix, allow) url rules sorted longest-prefix-first.
    url_rules: Vec<(String, bool)>,
    url_blanket_deny: bool,
}

fn str_field(resource: &Value, key: &str) -> String {
    resource
        .get(key)
        .and_then(|v| v.as_str())
        .unwrap_or_default()
        .trim()
        .to_string()
}

fn op_of(part: &str) -> Option<Op> {
    match part {
        "read" => Some(Op::Read),
        "create" => Some(Op::Create),
        "update" | "edit" => Some(Op::Update),
        "delete" | "retire" => Some(Op::Delete),
        _ => None,
    }
}

impl DataPerm {
    /// Load and compile all rules of the principal's effective groups
    /// (direct memberships + ancestor groups through parent_id).
    pub async fn load(
        tx: &mut Transaction<'_, Postgres>,
        tenant: uuid::Uuid,
        principal: uuid::Uuid,
    ) -> Result<DataPerm, DbError> {
        let db = |e: sqlx::Error| DbError::Other {
            message: e.to_string(),
        };
        let rows = sqlx::query(
            "WITH RECURSIVE eff AS (
                 SELECT g.group_id, g.alias, g.parent_id
                 FROM sec.principal_group_membership m
                 JOIN sec.\"group\" g ON g.group_id = m.group_id
                 WHERE m.tenant_id = $1 AND m.principal_id = $2
                   AND (m.valid_to IS NULL OR m.valid_to >= now())
                   AND (m.valid_from <= now() + interval '120 seconds')
                 UNION
                 SELECT p.group_id, p.alias, p.parent_id
                 FROM sec.\"group\" p
                 JOIN eff e ON e.parent_id = p.group_id
                 WHERE p.tenant_id = $1
             )
             SELECT DISTINCT r.action, r.effect, r.resource,
                    COALESCE(r.field_mask, '') AS field_mask
             FROM sec.policy_rule r
             WHERE r.tenant_id = $1
               AND r.group_id IN (SELECT group_id FROM eff)
               AND r.valid_from <= now() + interval '120 seconds'
               AND (r.valid_to IS NULL OR r.valid_to >= now())",
        )
        .bind(tenant)
        .bind(principal)
        .fetch_all(&mut **tx)
        .await
        .map_err(&db)?;

        let flags: Vec<String> = sqlx::query_scalar(
            "WITH RECURSIVE eff AS (
                 SELECT g.group_id, g.parent_id
                 FROM sec.principal_group_membership m
                 JOIN sec.\"group\" g ON g.group_id = m.group_id
                 WHERE m.tenant_id = $1 AND m.principal_id = $2
                   AND (m.valid_to IS NULL OR m.valid_to >= now())
                   AND (m.valid_from <= now() + interval '120 seconds')
                 UNION
                 SELECT p.group_id, p.parent_id
                 FROM sec.\"group\" p JOIN eff e ON e.parent_id = p.group_id
                 WHERE p.tenant_id = $1
             )
             SELECT DISTINCT COALESCE(NULLIF(g.special,''), g.alias)
             FROM sec.\"group\" g
             WHERE g.group_id IN (SELECT group_id FROM eff)
               AND COALESCE(NULLIF(g.special,''), g.alias)
                   IN ('superadmin','superuser')",
        )
        .bind(tenant)
        .bind(principal)
        .fetch_all(&mut **tx)
        .await
        .map_err(&db)?;

        let mut perm = DataPerm::from_flags(&flags);
        for row in &rows {
            let action: String = row.try_get("action").unwrap_or_default();
            let effect: String = row.try_get("effect").unwrap_or_default();
            let resource: Value = row.try_get::<Value, _>("resource").unwrap_or(Value::Null);
            let mask: String = row.try_get("field_mask").unwrap_or_default();
            perm.apply_rule(&action, &effect, &resource, &mask);
        }
        perm.finish();
        Ok(perm)
    }

    /// Load in a standalone transaction (used by the request guard, which
    /// runs before any handler transaction exists).
    pub async fn load_exec(
        exec: &std::sync::Arc<enginer_db_runtime::AsyncStdDbExecutor>,
        tenant: uuid::Uuid,
        principal: uuid::Uuid,
    ) -> Result<DataPerm, DbError> {
        use enginer_db_runtime::Deadline;
        let v = exec
            .run(Deadline::in_secs(5), move |pool| async move {
                let mut tx = pool.begin().await.map_err(|e| DbError::Other {
                    message: e.to_string(),
                })?;
                sqlx::query("SELECT set_config('app.tenant_id',$1,true)")
                    .bind(tenant.to_string())
                    .execute(&mut *tx)
                    .await
                    .map_err(|e| DbError::Other {
                        message: e.to_string(),
                    })?;
                let rows = sqlx::query(
                    "WITH RECURSIVE eff AS (
                         SELECT g.group_id, g.alias, g.parent_id
                         FROM sec.principal_group_membership m
                         JOIN sec.\"group\" g ON g.group_id = m.group_id
                         WHERE m.tenant_id = $1 AND m.principal_id = $2
                           AND (m.valid_to IS NULL OR m.valid_to >= now())
                           AND (m.valid_from <= now() + interval '120 seconds')
                         UNION
                         SELECT p.group_id, p.alias, p.parent_id
                         FROM sec.\"group\" p
                         JOIN eff e ON e.parent_id = p.group_id
                         WHERE p.tenant_id = $1
                     )
                     SELECT DISTINCT r.action, r.effect, r.resource,
                            COALESCE(r.field_mask, '') AS field_mask
                     FROM sec.policy_rule r
                     WHERE r.tenant_id = $1
                       AND r.group_id IN (SELECT group_id FROM eff)
                       AND r.valid_from <= now() + interval '120 seconds'
                       AND (r.valid_to IS NULL OR r.valid_to >= now())",
                )
                .bind(tenant)
                .bind(principal)
                .fetch_all(&mut *tx)
                .await
                .map_err(|e| DbError::Other {
                    message: e.to_string(),
                })?;
                let flags: Vec<String> = sqlx::query_scalar(
                    "WITH RECURSIVE eff AS (
                         SELECT g.group_id, g.parent_id
                         FROM sec.principal_group_membership m
                         JOIN sec.\"group\" g ON g.group_id = m.group_id
                         WHERE m.tenant_id = $1 AND m.principal_id = $2
                           AND (m.valid_to IS NULL OR m.valid_to >= now())
                           AND (m.valid_from <= now() + interval '120 seconds')
                         UNION
                         SELECT p.group_id, p.parent_id
                         FROM sec.\"group\" p JOIN eff e ON e.parent_id = p.group_id
                         WHERE p.tenant_id = $1
                     )
                     SELECT DISTINCT COALESCE(NULLIF(g.special,''), g.alias)
                     FROM sec.\"group\" g
                     WHERE g.group_id IN (SELECT group_id FROM eff)
                       AND COALESCE(NULLIF(g.special,''), g.alias)
                           IN ('superadmin','superuser')",
                )
                .bind(tenant)
                .bind(principal)
                .fetch_all(&mut *tx)
                .await
                .map_err(|e| DbError::Other {
                    message: e.to_string(),
                })?;
                tx.commit().await.map_err(|e| DbError::Other {
                    message: e.to_string(),
                })?;
                let rules: Vec<Value> = rows
                    .iter()
                    .map(|r| {
                        serde_json::json!({
                            "action": r.try_get::<String, _>("action").unwrap_or_default(),
                            "effect": r.try_get::<String, _>("effect").unwrap_or_default(),
                            "resource": r.try_get::<Value, _>("resource").unwrap_or(Value::Null),
                            "field_mask": r.try_get::<String, _>("field_mask").unwrap_or_default(),
                        })
                    })
                    .collect();
                Ok(serde_json::json!({ "flags": flags, "rules": rules }))
            })
            .await?;
        let mut perm = DataPerm::from_flags(
            &v["flags"]
                .as_array()
                .map(|a| {
                    a.iter()
                        .filter_map(|x| x.as_str().map(str::to_string))
                        .collect::<Vec<_>>()
                })
                .unwrap_or_default(),
        );
        for r in v["rules"].as_array().cloned().unwrap_or_default() {
            perm.apply_rule(
                r["action"].as_str().unwrap_or_default(),
                r["effect"].as_str().unwrap_or_default(),
                &r["resource"],
                r["field_mask"].as_str().unwrap_or_default(),
            );
        }
        perm.finish();
        Ok(perm)
    }

    fn from_flags(flags: &[String]) -> DataPerm {
        DataPerm {
            superadmin: flags.iter().any(|a| a == "superadmin"),
            superuser: flags.iter().any(|a| a == "superuser"),
            ..Default::default()
        }
    }

    fn finish(&mut self) {
        self.url_rules.sort_by(|a, b| b.0.len().cmp(&a.0.len()));
    }

    fn apply_rule(&mut self, action: &str, effect: &str, resource: &Value, mask: &str) {
        if action == "system.url" {
            let path = str_field(resource, "path");
            let path = if path.is_empty() {
                "/".to_string()
            } else {
                path
            };
            if effect == "deny" && (path == "/" || path == "*") {
                self.url_blanket_deny = true;
            } else {
                // An allow rule on '/' allowlists everything (longest-prefix).
                self.url_rules.push((path, effect != "deny"));
            }
            return;
        }
        if action == "data.*" || action == "data.all.*" {
            if effect != "deny" {
                self.wildcard_allow = true;
            } else {
                // Deny-overrides: `deny data.*` is a total data deny, not just
                // a revocation of another group's `allow data.*` — otherwise a
                // `data.all.<op>` allow would survive the blanket deny.
                self.wildcard_allow = false;
                self.all_rules.apply_all("deny");
            }
            return;
        }
        let Some(rest) = action.strip_prefix("data.") else {
            return;
        };
        let mut it = rest.splitn(2, '.');
        let (Some(scope), Some(op_s)) = (it.next(), it.next()) else {
            return;
        };
        let all_ops = op_s == "*";
        let Some(op) = (if all_ops { Some(Op::Read) } else { op_of(op_s) }) else {
            return;
        };
        match scope {
            "all" => {
                if all_ops {
                    self.all_rules.apply_all(effect);
                } else {
                    self.all_rules.apply(op, effect);
                }
            }
            "type" | "entity_type" => {
                let key = str_field(resource, "type");
                let set = self.type_rules.entry(key).or_default();
                if all_ops {
                    set.apply_all(effect);
                } else {
                    set.apply(op, effect);
                }
            }
            "field" => {
                let key = (str_field(resource, "type"), str_field(resource, "field"));
                let set = self.field_rules.entry(key.clone()).or_default();
                if all_ops {
                    set.apply_all(effect);
                } else {
                    set.apply(op, effect);
                }
                if effect == "allow" && !mask.is_empty() {
                    self.field_masks.insert(key, mask.to_string());
                }
            }
            "entity" | "object" => {
                let key = str_field(resource, "entity");
                if key.is_empty() {
                    return;
                }
                let set = self.entity_rules.entry(key).or_default();
                if all_ops {
                    set.apply_all(effect);
                } else {
                    set.apply(op, effect);
                }
            }
            "relation" | "predicate" => {
                let dir = match str_field(resource, "direction").as_str() {
                    "outgoing" | "out" => "out",
                    "incoming" | "in" => "in",
                    _ => "",
                };
                let key = (str_field(resource, "predicate"), dir.to_string());
                let set = self.rel_rules.entry(key).or_default();
                if all_ops {
                    set.apply_all(effect);
                } else {
                    set.apply(op, effect);
                }
            }
            "relation_instance" | "statement" => {
                let key = str_field(resource, "relation");
                if key.is_empty() {
                    return;
                }
                let set = self.rel_inst_rules.entry(key).or_default();
                if all_ops {
                    set.apply_all(effect);
                } else {
                    set.apply(op, effect);
                }
            }
            _ => {}
        }
    }

    /// CRUD on an object type (1.1) — default deny.
    /// `keys` = both the type uuid and its stable alias.
    pub fn can_type(&self, keys: &[&str], op: Op) -> bool {
        if self.superadmin || self.superuser {
            return true;
        }
        // Deny checks must include the wildcard keys too — a
        // `deny data.type.<op> {type:"*"}` rule overrides every specific
        // allow (deny-overrides), not just rules on the same key.
        for k in keys.iter().chain(["", "*"].iter()) {
            if let Some(s) = self.type_rules.get(*k) {
                if s.denied(op) {
                    return false;
                }
            }
        }
        if self.all_rules.denied(op) {
            return false;
        }
        if self.wildcard_allow || self.all_rules.granted(op) {
            return true;
        }
        for k in keys {
            if self.type_rules.get(*k).is_some_and(|s| s.granted(op)) {
                return true;
            }
        }
        self.type_rules
            .get("")
            .or_else(|| self.type_rules.get("*"))
            .is_some_and(|s| s.granted(op))
    }

    /// CRUD on a field of an object type (1.2) — default deny.
    /// Falls back to a wildcard field rule (field = '*'/'') for the type.
    pub fn can_field(&self, type_keys: &[&str], field: &str, op: Op) -> bool {
        if self.superadmin || self.superuser {
            return true;
        }
        if !self.can_type(type_keys, op) && op == Op::Read {
            return false;
        }
        // Type-level deny blocks every field op (deny-overrides), not only
        // reads — previously a `deny data.type.update` could be bypassed
        // by a field-level or wildcard allow.
        for tk in type_keys.iter().chain(["", "*"].iter()) {
            if let Some(s) = self.type_rules.get(*tk) {
                if s.denied(op) {
                    return false;
                }
            }
        }
        // Field denies win — specific type, then wildcard types ("*"/"").
        for tk in type_keys.iter().chain(["", "*"].iter()) {
            for fk in [field, "*", ""] {
                if let Some(s) = self.field_rules.get(&(tk.to_string(), fk.to_string())) {
                    if s.denied(op) {
                        return false;
                    }
                }
            }
        }
        if self.all_rules.denied(op) {
            return false;
        }
        if self.wildcard_allow || self.all_rules.granted(op) {
            return true;
        }
        for tk in type_keys {
            for fk in [field, "*", ""] {
                if self
                    .field_rules
                    .get(&(tk.to_string(), fk.to_string()))
                    .is_some_and(|s| s.granted(op))
                {
                    return true;
                }
            }
        }
        // A field rule with a wildcard type grants the field on every type.
        for tk in ["", "*"] {
            for fk in [field, "*"] {
                if self
                    .field_rules
                    .get(&(tk.to_string(), fk.to_string()))
                    .is_some_and(|s| s.granted(op))
                {
                    return true;
                }
            }
        }
        false
    }

    /// CRUD on a concrete entity instance (1.3) — default allow.
    pub fn can_entity(&self, public_id: &str, op: Op) -> bool {
        if self.superadmin || self.superuser {
            return true;
        }
        if self.all_rules.denied(op) {
            return false;
        }
        // Wildcard instance rules ("*"/"") deny every instance too.
        [public_id, "*", ""]
            .iter()
            .all(|k| self.entity_rules.get(*k).is_none_or(|s| !s.denied(op)))
    }

    /// CRUD on a relation type (predicate) (1.4) — default deny.
    /// Direction-agnostic check: consults only rules without a direction scope.
    /// Callers that know the edge direction should use `can_relation_dir`.
    #[allow(dead_code)]
    pub fn can_relation(&self, predicate: &str, op: Op) -> bool {
        self.can_relation_dir(predicate, op, None)
    }

    /// Direction-aware variant: `outgoing = Some(true)` checks rules for
    /// outgoing edges (plus direction-agnostic ones), `Some(false)` for
    /// incoming, `None` for rules that apply in both directions only.
    pub fn can_relation_dir(&self, predicate: &str, op: Op, outgoing: Option<bool>) -> bool {
        if self.superadmin || self.superuser {
            return true;
        }
        let dir = match outgoing {
            Some(true) => "out",
            Some(false) => "in",
            None => "",
        };
        let dirs: [&str; 2] = ["", dir];
        for p in [predicate, "*", ""] {
            for d in dirs {
                if let Some(s) = self.rel_rules.get(&(p.to_string(), d.to_string())) {
                    if s.denied(op) {
                        return false;
                    }
                }
            }
        }
        if self.all_rules.denied(op) {
            return false;
        }
        if self.wildcard_allow || self.all_rules.granted(op) {
            return true;
        }
        [predicate, "*", ""].iter().any(|p| {
            dirs.iter().any(|d| {
                self.rel_rules
                    .get(&(p.to_string(), d.to_string()))
                    .is_some_and(|s| s.granted(op))
            })
        })
    }

    /// CRUD on a concrete relation instance (1.5) — default allow.
    pub fn can_relation_instance(&self, relation_id: &str, op: Op) -> bool {
        if self.superadmin || self.superuser {
            return true;
        }
        if self.all_rules.denied(op) {
            return false;
        }
        [relation_id, "*", ""]
            .iter()
            .all(|k| self.rel_inst_rules.get(*k).is_none_or(|s| !s.denied(op)))
    }

    /// URL access (2.1/2.2): superuser loses admin URL prefixes;
    /// deny rules block; a blanket deny turns access into an allowlist.
    pub fn can_url(&self, path: &str) -> bool {
        if self.superadmin {
            return true;
        }
        if self.superuser {
            // Admin URL prefixes apply to both UI paths (/admin/users) and
            // their API counterparts (/api/v1/users) — normalize both forms.
            let short = path.strip_prefix("/api/v1").unwrap_or(path);
            if ADMIN_URL_PREFIXES.iter().any(|p| {
                path == *p
                    || path.starts_with(&format!("{p}/"))
                    || short == *p
                    || short.starts_with(&format!("{p}/"))
            }) {
                return false;
            }
        }
        for (prefix, allow) in &self.url_rules {
            // Longest-prefix match; '/' matches every path.
            if path == *prefix
                || prefix == "/"
                || *prefix == "*"
                || path.starts_with(&format!("{prefix}/"))
            {
                return *allow;
            }
        }
        !self.url_blanket_deny
    }

    /// Capability check with special roles applied.
    pub fn can_capability(&self, action: &str) -> Option<bool> {
        if self.superadmin {
            return Some(true);
        }
        if self.superuser {
            return Some(!ADMIN_CAPS.contains(&action));
        }
        None
    }

    /// Strip fields the principal cannot read from a field map.
    pub fn filter_fields(&self, type_keys: &[&str], fields: &mut serde_json::Map<String, Value>) {
        if self.superadmin || self.superuser {
            return;
        }
        fields.retain(|k, _| self.can_field(type_keys, k, Op::Read));
    }

    /// Mask of a field that is readable but redacted (`policy_rule.field_mask`).
    /// `mask` survives even when the ruleset uses a type wildcard.
    pub fn mask_for(&self, type_keys: &[&str], field: &str) -> Option<&str> {
        if self.superadmin || self.superuser {
            return None;
        }
        for tk in type_keys {
            for fk in [field, "*", ""] {
                if let Some(m) = self.field_masks.get(&(tk.to_string(), fk.to_string())) {
                    return Some(m.as_str());
                }
            }
        }
        // A mask can also be defined across all types.
        for fk in [field, "*", ""] {
            for tk in ["*", ""] {
                if let Some(m) = self.field_masks.get(&(tk.to_string(), fk.to_string())) {
                    return Some(m.as_str());
                }
            }
        }
        None
    }

    /// Transform readable-but-masked fields in place: `omit` drops the key,
    /// `null-mask` nulls it, `partial-mask` keeps the outer characters.
    pub fn mask_fields(&self, type_keys: &[&str], fields: &mut serde_json::Map<String, Value>) {
        if self.superadmin || self.superuser {
            return;
        }
        let keys: Vec<String> = fields.keys().cloned().collect();
        for k in keys {
            let Some(mask) = self.mask_for(type_keys, &k) else {
                continue;
            };
            match mask {
                "omit" => {
                    fields.remove(&k);
                }
                "null-mask" => {
                    fields.insert(k, Value::Null);
                }
                "partial-mask" => {
                    let masked = fields
                        .get(&k)
                        .and_then(|v| v.as_str())
                        .map(partial_mask)
                        .unwrap_or_else(|| "***".to_string());
                    fields.insert(k, Value::String(masked));
                }
                // Deterministic pseudonymization: equal values map to equal
                // tokens without revealing the cleartext. Strings hash without
                // the JSON quotes; other kinds use their JSON representation.
                "tokenize" | "token" => {
                    let token = fields
                        .get(&k)
                        .map(|v| match v.as_str() {
                            Some(s) => tokenize(s),
                            None => tokenize(&v.to_string()),
                        })
                        .unwrap_or_else(|| tokenize(&k));
                    fields.insert(k, Value::String(token));
                }
                _ => {}
            }
        }
    }
}

/// Deterministic token for the `tokenize` mask — stable pseudonym, not PII.
fn tokenize(s: &str) -> String {
    use sha2::{Digest, Sha256};
    let digest = Sha256::digest(s.as_bytes());
    format!("tok_{}", &hex_lower(&digest[..8]))
}

fn hex_lower(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

/// Keep the outer two characters of a string, mask the middle.
fn partial_mask(s: &str) -> String {
    let chars: Vec<char> = s.chars().collect();
    if chars.len() <= 4 {
        return "*".repeat(chars.len());
    }
    format!(
        "{}{}{}",
        chars[..2].iter().collect::<String>(),
        "***",
        chars[chars.len() - 2..].iter().collect::<String>()
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn rule(perm: &mut DataPerm, action: &str, effect: &str, resource: Value) {
        perm.apply_rule(action, effect, &resource, "");
    }

    fn masked_rule(perm: &mut DataPerm, action: &str, effect: &str, resource: Value, mask: &str) {
        perm.apply_rule(action, effect, &resource, mask);
    }

    #[test]
    fn type_default_deny_and_grants() {
        let mut p = DataPerm::default();
        assert!(!p.can_type(&["person"], Op::Read));
        rule(&mut p, "data.type.read", "allow", json!({"type":"person"}));
        assert!(p.can_type(&["person"], Op::Read));
        assert!(!p.can_type(&["person"], Op::Update));
        assert!(!p.can_type(&["legal_entity"], Op::Read));
        // deny overrides
        rule(&mut p, "data.type.read", "deny", json!({"type":"person"}));
        assert!(!p.can_type(&["person"], Op::Read));
    }

    #[test]
    fn type_wildcard_rule_covers_all_types() {
        let mut p = DataPerm::default();
        rule(&mut p, "data.type.read", "allow", json!({"type":"*"}));
        assert!(p.can_type(&["anything"], Op::Read));
        assert!(!p.can_type(&["anything"], Op::Delete));
    }

    #[test]
    fn data_star_grants_everything() {
        let mut p = DataPerm::default();
        rule(&mut p, "data.*", "allow", json!({}));
        assert!(p.can_type(&["x"], Op::Delete));
        assert!(p.can_field(&["x"], "f", Op::Read));
        assert!(p.can_relation("works_for", Op::Create));
    }

    #[test]
    fn deny_data_star_is_total_deny_not_just_wildcard_revocation() {
        let mut p = DataPerm::default();
        rule(&mut p, "data.all.read", "allow", json!({}));
        rule(&mut p, "data.type.read", "allow", json!({"type":"person"}));
        rule(&mut p, "data.*", "deny", json!({}));
        // deny data.* beats every allow, including data.all.* and per-scope.
        assert!(!p.can_type(&["person"], Op::Read));
        assert!(!p.can_field(&["person"], "f", Op::Read));
        assert!(!p.can_relation("owns", Op::Read));
        assert!(!p.can_entity("e-1", Op::Read));
        assert!(!p.can_entity("e-1", Op::Delete));
    }

    #[test]
    fn field_default_deny_with_wildcard_and_deny() {
        let mut p = DataPerm::default();
        rule(&mut p, "data.type.read", "allow", json!({"type":"person"}));
        rule(
            &mut p,
            "data.field.read",
            "allow",
            json!({"type":"person","field":"*"}),
        );
        assert!(p.can_field(&["person"], "name", Op::Read));
        rule(
            &mut p,
            "data.field.read",
            "deny",
            json!({"type":"person","field":"salary"}),
        );
        assert!(!p.can_field(&["person"], "salary", Op::Read));
        assert!(!p.can_field(&["company"], "name", Op::Read));
    }

    #[test]
    fn entity_instance_default_allow_with_deny() {
        let mut p = DataPerm::default();
        assert!(p.can_entity("e-1", Op::Read));
        rule(&mut p, "data.entity.read", "deny", json!({"entity":"e-1"}));
        assert!(!p.can_entity("e-1", Op::Read));
        assert!(p.can_entity("e-1", Op::Update));
        assert!(p.can_entity("e-2", Op::Read));
    }

    #[test]
    fn relation_type_default_deny() {
        let mut p = DataPerm::default();
        assert!(!p.can_relation("employs", Op::Read));
        rule(
            &mut p,
            "data.relation.read",
            "allow",
            json!({"predicate":"employs"}),
        );
        assert!(p.can_relation("employs", Op::Read));
        assert!(!p.can_relation("owns", Op::Read));
    }

    #[test]
    fn relation_instance_default_allow() {
        let mut p = DataPerm::default();
        assert!(p.can_relation_instance("rel-9", Op::Update));
        rule(
            &mut p,
            "data.relation_instance.update",
            "deny",
            json!({"relation":"rel-9"}),
        );
        assert!(!p.can_relation_instance("rel-9", Op::Update));
        assert!(p.can_relation_instance("rel-9", Op::Read));
    }

    #[test]
    fn superadmin_and_superuser() {
        let mut p = DataPerm::default();
        p.superadmin = true;
        assert!(p.can_url("/permissions/groups"));
        assert_eq!(p.can_capability("users.manage"), Some(true));
        let mut p = DataPerm::default();
        p.superuser = true;
        assert!(p.can_type(&["x"], Op::Read));
        assert!(p.can_url("/entities/e1"));
        assert!(!p.can_url("/users"));
        assert!(!p.can_url("/api/v1/users")); // API form of the admin surface
        assert!(!p.can_url("/api/v1/permissions/groups"));
        assert_eq!(p.can_capability("users.manage"), Some(false));
        assert_eq!(p.can_capability("entity.read"), Some(true));
    }

    #[test]
    fn url_rules_prefix_longest_match() {
        let mut p = DataPerm::default();
        rule(&mut p, "system.url", "deny", json!({"path":"/"}));
        rule(&mut p, "system.url", "allow", json!({"path":"/entities"}));
        rule(
            &mut p,
            "system.url",
            "deny",
            json!({"path":"/entities/secret"}),
        );
        p.url_rules.sort_by(|a, b| b.0.len().cmp(&a.0.len()));
        assert!(p.can_url("/entities/e1"));
        assert!(!p.can_url("/entities/secret"));
        assert!(!p.can_url("/reports"));
    }

    #[test]
    fn field_masks_omit_null_partial() {
        let mut p = DataPerm::default();
        rule(&mut p, "data.type.read", "allow", json!({"type":"person"}));
        masked_rule(
            &mut p,
            "data.field.read",
            "allow",
            json!({"type":"person","field":"*"}),
            "",
        );
        masked_rule(
            &mut p,
            "data.field.read",
            "allow",
            json!({"type":"person","field":"inn"}),
            "partial-mask",
        );
        masked_rule(
            &mut p,
            "data.field.read",
            "allow",
            json!({"type":"person","field":"snils"}),
            "null-mask",
        );
        masked_rule(
            &mut p,
            "data.field.read",
            "allow",
            json!({"type":"person","field":"salary"}),
            "omit",
        );
        let mut fields = serde_json::Map::new();
        fields.insert("name".to_string(), json!("Иван Иванов"));
        fields.insert("inn".to_string(), json!("7712345678"));
        fields.insert("snils".to_string(), json!("123-456-789"));
        fields.insert("salary".to_string(), json!(100));
        p.mask_fields(&["person"], &mut fields);
        assert_eq!(fields["name"], json!("Иван Иванов"));
        assert_eq!(fields["inn"], json!("77***78"));
        assert_eq!(fields["snils"], Value::Null);
        assert!(!fields.contains_key("salary"));
        // Masked fields are still readable — the mask is not a deny.
        assert!(p.can_field(&["person"], "salary", Op::Read));
    }

    #[test]
    fn field_mask_not_applied_for_special_roles() {
        let mut p = DataPerm::default();
        masked_rule(
            &mut p,
            "data.field.read",
            "allow",
            json!({"type":"person","field":"inn"}),
            "null-mask",
        );
        p.superuser = true;
        assert_eq!(p.mask_for(&["person"], "inn"), None);
    }

    #[test]
    fn user_admin_caps_are_administrative() {
        let mut p = DataPerm::default();
        p.superuser = true;
        for cap in [
            "user.read",
            "user.create",
            "user.edit",
            "user.delete",
            "users.manage",
        ] {
            assert_eq!(p.can_capability(cap), Some(false), "{cap}");
        }
        assert_eq!(p.can_capability("entity.read"), Some(true));
    }

    #[test]
    fn entity_deny_hides_relations_of_that_entity() {
        // Instance-level deny (1.3) also hides every relation involving the
        // entity — the relation check must pass can_entity of both sides.
        let mut p = DataPerm::default();
        rule(
            &mut p,
            "data.relation.read",
            "allow",
            json!({"predicate":"*"}),
        );
        rule(
            &mut p,
            "data.entity.read",
            "deny",
            json!({"entity":"hidden-1"}),
        );
        assert!(p.can_relation("employs", Op::Read));
        // Handler-level: edge listed only if can_entity(other_side) — denied.
        assert!(!p.can_entity("hidden-1", Op::Read));
    }

    #[test]
    fn relation_default_denied_when_object_type_denied() {
        // 1.6: without type access, relations to that type are denied unless
        // a relation rule explicitly allows them.
        let mut p = DataPerm::default();
        assert!(!p.can_type(&["person"], Op::Read));
        assert!(!p.can_relation("person.works_for", Op::Read));
        // Explicit relation grant still applies at the relation scope —
        // the endpoint additionally requires entity visibility, so the grant
        // alone cannot expose the target.
        rule(
            &mut p,
            "data.relation.read",
            "allow",
            json!({"predicate":"person.works_for"}),
        );
        assert!(p.can_relation("person.works_for", Op::Read));
    }

    #[test]
    fn object_edit_does_not_imply_relation_edit() {
        let mut p = DataPerm::default();
        rule(
            &mut p,
            "data.type.update",
            "allow",
            json!({"type":"person"}),
        );
        assert!(p.can_type(&["person"], Op::Update));
        assert!(!p.can_relation("person.works_for", Op::Update));
    }

    #[test]
    fn object_edit_does_not_imply_field_edit() {
        let mut p = DataPerm::default();
        rule(
            &mut p,
            "data.type.update",
            "allow",
            json!({"type":"person"}),
        );
        assert!(!p.can_field(&["person"], "inn", Op::Update));
        rule(
            &mut p,
            "data.field.update",
            "allow",
            json!({"type":"person","field":"inn"}),
        );
        assert!(p.can_field(&["person"], "inn", Op::Update));
    }

    #[test]
    fn relation_direction_rules_scope_access() {
        let mut p = DataPerm::default();
        // Allow reading `employs` only in the outgoing direction.
        rule(
            &mut p,
            "data.relation.read",
            "allow",
            json!({"predicate":"employs","direction":"outgoing"}),
        );
        assert!(p.can_relation_dir("employs", Op::Read, Some(true)));
        assert!(!p.can_relation_dir("employs", Op::Read, Some(false)));
        // Direction-agnostic check does not see direction-scoped allows.
        assert!(!p.can_relation("employs", Op::Read));

        // Incoming-scoped deny blocks incoming even when a 'both' allow exists.
        rule(
            &mut p,
            "data.relation.read",
            "allow",
            json!({"predicate":"owns"}),
        );
        assert!(p.can_relation_dir("owns", Op::Read, Some(true)));
        assert!(p.can_relation_dir("owns", Op::Read, Some(false)));
        rule(
            &mut p,
            "data.relation.read",
            "deny",
            json!({"predicate":"owns","direction":"incoming"}),
        );
        assert!(p.can_relation_dir("owns", Op::Read, Some(true)));
        assert!(!p.can_relation_dir("owns", Op::Read, Some(false)));
    }

    #[test]
    fn wildcard_type_deny_overrides_specific_allow() {
        // deny data.type.read {type:"*"} must beat allow on a concrete type.
        let mut p = DataPerm::default();
        rule(&mut p, "data.type.read", "deny", json!({"type":"*"}));
        rule(&mut p, "data.type.read", "allow", json!({"type":"person"}));
        assert!(!p.can_type(&["person"], Op::Read));
        assert!(!p.can_type(&["other"], Op::Read));
        // Reads are off, writes still follow their own rules.
        rule(
            &mut p,
            "data.type.update",
            "allow",
            json!({"type":"person"}),
        );
        assert!(p.can_type(&["person"], Op::Update));
    }

    #[test]
    fn type_deny_blocks_field_ops_not_just_read() {
        // deny data.type.update + allow data.field.update must not write.
        let mut p = DataPerm::default();
        rule(&mut p, "data.type.update", "deny", json!({"type":"person"}));
        rule(
            &mut p,
            "data.field.update",
            "allow",
            json!({"type":"person","field":"inn"}),
        );
        assert!(!p.can_field(&["person"], "inn", Op::Update));
        // wildcard type deny also blocks fields on any type.
        let mut p = DataPerm::default();
        rule(&mut p, "data.type.update", "deny", json!({"type":"*"}));
        rule(&mut p, "data.*", "allow", json!({}));
        assert!(!p.can_field(&["person"], "inn", Op::Update));
    }

    #[test]
    fn wildcard_field_deny_applies_to_every_type() {
        let mut p = DataPerm::default();
        rule(&mut p, "data.type.read", "allow", json!({"type":"person"}));
        rule(
            &mut p,
            "data.field.read",
            "allow",
            json!({"type":"person","field":"*"}),
        );
        rule(
            &mut p,
            "data.field.read",
            "deny",
            json!({"type":"*","field":"salary"}),
        );
        assert!(!p.can_field(&["person"], "salary", Op::Read));
        assert!(p.can_field(&["person"], "name", Op::Read));
    }

    #[test]
    fn wildcard_entity_deny_hides_every_instance() {
        let mut p = DataPerm::default();
        rule(&mut p, "data.entity.read", "deny", json!({"entity":"*"}));
        assert!(!p.can_entity("e-1", Op::Read));
        assert!(p.can_entity("e-1", Op::Update)); // op still defaults allow
    }

    #[test]
    fn tokenize_mask_is_deterministic() {
        let mut p = DataPerm::default();
        rule(&mut p, "data.type.read", "allow", json!({"type":"person"}));
        masked_rule(
            &mut p,
            "data.field.read",
            "allow",
            json!({"type":"person","field":"inn"}),
            "tokenize",
        );
        let mut f1 = serde_json::Map::new();
        f1.insert("inn".into(), json!("7707083893"));
        let mut f2 = f1.clone();
        p.mask_fields(&["person"], &mut f1);
        p.mask_fields(&["person"], &mut f2);
        let t1 = f1.get("inn").unwrap().as_str().unwrap().to_string();
        assert!(t1.starts_with("tok_"));
        assert_eq!(t1, f2.get("inn").unwrap().as_str().unwrap());
        assert_ne!(t1, "7707083893");
    }
}
