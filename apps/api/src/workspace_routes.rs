//! Shared route scaffolding: PEP `CapabilityGuard`, API envelope helpers and
//! wire records reused by the PostgreSQL-backed route modules.
//!
//! All former in-memory handlers and `WorkspaceStore` state have been
//! superseded by `entity_pg`, `skr_pg`, `files_pg`, `audit_pg` and
//! `schema_admin_routes`.

use rocket::http::Status;
use rocket::request::{self, FromRequest, Request};
use serde::{Deserialize, Serialize};
use serde_json::Value;

/// PEP request guard (docs/08 §4): resolves the `session` cookie into an
/// `AuthContext`, enforces CSRF on mutations, and evaluates the route's
/// required capability through the PDP (deny-overrides, fail closed).
///
/// Handlers that need the authenticated principal can read `cap.ctx`.
#[derive(Debug)]
pub struct CapabilityGuard {
    /// Resolved principal/session; handlers use it for tenant-scoped queries.
    #[allow(dead_code)]
    pub ctx: crate::authn::AuthContext,
}

#[rocket::async_trait]
impl<'r> FromRequest<'r> for CapabilityGuard {
    type Error = &'static str;

    async fn from_request(req: &'r Request<'_>) -> request::Outcome<Self, Self::Error> {
        use request::Outcome;
        let Some(state) = req.rocket().state::<crate::AppState>() else {
            return Outcome::Error((Status::InternalServerError, "no app state"));
        };
        let Some(exec) = state.db_executor.clone() else {
            return Outcome::Error((Status::ServiceUnavailable, "db unavailable"));
        };
        // Two credential channels (docs/08 §2):
        //   * `session` cookie — human/UI sessions.
        //   * `Authorization: Bearer egr_…` — API tokens bound to a
        //     (usually service) principal (docs/09 §3).
        // Bearer wins when present so programmatic callers are not confused
        // by a stale cookie on the same connection.
        let bearer = req
            .headers()
            .get_one("authorization")
            .and_then(|h| h.strip_prefix("Bearer "))
            .map(str::trim)
            .filter(|t| t.starts_with(crate::authn::API_TOKEN_PREFIX))
            .map(str::to_string);
        let ctx = if let Some(token) = bearer {
            match crate::authn::resolve_bearer(&exec, &token).await {
                Ok(Some(ctx)) => ctx,
                Ok(None) => return Outcome::Error((Status::Unauthorized, "invalid token")),
                Err(_) => {
                    return Outcome::Error((Status::ServiceUnavailable, "auth backend error"))
                }
            }
        } else {
            let Some(cookie) = req.cookies().get("session") else {
                return Outcome::Error((Status::Unauthorized, "missing session"));
            };
            let Ok(session_id) = cookie.value().parse::<uuid::Uuid>() else {
                return Outcome::Error((Status::Unauthorized, "bad session"));
            };
            match crate::authn::resolve_session(&exec, session_id).await {
                Ok(Some(ctx)) => ctx,
                Ok(None) => return Outcome::Error((Status::Unauthorized, "invalid session")),
                Err(_) => {
                    return Outcome::Error((Status::ServiceUnavailable, "auth backend error"))
                }
            }
        };

        // CSRF (docs/16 §13): cookie-authenticated mutations must echo the
        // per-session token issued at login. Bearer tokens are not ambient
        // credentials — a browser never attaches them on its own — so the
        // CSRF echo does not apply to them.
        if !ctx.via_bearer
            && !matches!(
                req.method(),
                rocket::http::Method::Get
                    | rocket::http::Method::Head
                    | rocket::http::Method::Options
            )
        {
            let presented = req.headers().get_one("x-csrf-token").unwrap_or_default();
            if presented.is_empty() || presented != ctx.csrf_token {
                return Outcome::Error((Status::Forbidden, "csrf token mismatch"));
            }
        }

        // Stash the resolved context so the audit fairing can attribute
        // the request to this principal after the handler completes.
        req.local_cache(|| Some(ctx.clone()));

        // Compile the principal's data/system permission set once per request:
        // effective groups include ancestors (role tree), special roles
        // (superadmin/superuser) override, URL rules apply to every route.
        let path = req.uri().path().to_string();
        let perm = match crate::dataperm::DataPerm::load_exec(
            &exec,
            ctx.tenant_id,
            ctx.principal_id,
        )
        .await
        {
            Ok(p) => p,
            Err(_) => return Outcome::Error((Status::ServiceUnavailable, "perm backend error")),
        };
        let deny = |reason: &'static str, detail: String| {
            let audit_ctx = ctx.clone();
            let exec2 = exec.clone();
            rocket::tokio::spawn(crate::audit_pg::record_audit_event(
                exec2,
                audit_ctx.tenant_id,
                Some(audit_ctx.principal_id),
                Some(audit_ctx.session_id),
                "denied_request",
                reason.to_string(),
                "http",
                None,
                Some("deny"),
                "denied",
                None,
                serde_json::json!({ "path": detail }),
            ));
            Outcome::Error((Status::Forbidden, reason))
        };
        if !perm.can_url(&path) {
            return deny("url access denied", path);
        }
        // Acceptable capabilities for this route: usually a single cap, but
        // some surfaces also accept an umbrella grant (e.g. `users.manage`
        // covers the granular user.* actions).
        let caps = required_capabilities(req.method().as_str(), req.uri().path().as_str());
        for cap in &caps {
            match perm.can_capability(cap) {
                Some(true) => return Outcome::Success(CapabilityGuard { ctx }),
                Some(false) => continue, // special role denied this cap
                None => match crate::authn::decide_action(&exec, &ctx, cap, "http", None).await {
                    Ok(true) => return Outcome::Success(CapabilityGuard { ctx }),
                    Ok(false) => {}
                    Err(_) => {
                        return Outcome::Error((Status::ServiceUnavailable, "pdp unavailable"))
                    }
                },
            }
        }
        if let Some(first) = caps.first() {
            deny("missing capability", first.to_string())
        } else {
            // Route has no mapped capability — a valid session is enough.
            Outcome::Success(CapabilityGuard { ctx })
        }
    }
}

/// Map method+path to the capabilities accepted by the PDP
/// (`enginer_security_domain::CapabilityAction` wire strings). The request is
/// authorized when ANY listed capability is granted — the first element is
/// the canonical requirement, later entries are umbrella grants. Routes not
/// listed here still require a valid session via the guard.
fn required_capabilities(method: &str, path: &str) -> Vec<&'static str> {
    let segs: Vec<&str> = path.split('/').collect();
    let one = |c: &'static str| vec![c];
    match (method, &segs[..]) {
        // Database dump: full-database export — privileged operation.
        (_, ["", "api", "v1", "admin", "dump", ..]) => one("security.manage"),

        // Audit: read vs administer (checkpoint/retention are security ops).
        ("GET", ["", "api", "v1", "audit", ..]) => one("audit.read"),
        (_, ["", "api", "v1", "audit", ..]) => one("security.manage"),

        // Checks: view catalog/instances, run new checks.
        ("GET", ["", "api", "v1", "checks", ..]) => one("check.view"),
        ("POST", ["", "api", "v1", "checks"]) => one("check.run"),
        (_, ["", "api", "v1", "checks", ..]) => one("check.run"),

        // Connectors
        (_, ["", "api", "v1", "connectors", ..]) => one("connector.manage"),

        // Schema lifecycle: authoring and publication are separate capabilities.
        // The published type/predicate catalog is also the metadata source for
        // entity forms — read access needs entity.read, mutations need the
        // schema capabilities.
        ("POST", ["", "api", "v1", "schemas", "drafts", _, "publish"]) => one("schema.publish"),
        ("POST", ["", "api", "v1", "schemas", "revisions", _, "retire"]) => one("schema.publish"),
        ("GET", ["", "api", "v1", "schemas", "types", ..]) => one("entity.read"),
        ("GET", ["", "api", "v1", "schemas", "predicates", ..]) => one("entity.read"),
        // Published revision metadata is the same read-only catalog as the
        // type/predicate listings — readable wherever entity.read is.
        ("GET", ["", "api", "v1", "schemas", "published"]) => one("entity.read"),
        ("GET", ["", "api", "v1", "schemas", "revisions", ..]) => one("entity.read"),
        (_, ["", "api", "v1", "schemas", ..]) => one("schema.draft"),

        // Permission groups / admin pages: viewing needs the admin surface,
        // mutating roles/rules needs the security capability.
        ("GET", ["", "api", "v1", "permissions", ..]) => one("admin.pages.access"),
        (_, ["", "api", "v1", "permissions", ..]) => one("security.manage"),

        // User administration (1.10): granular CRUD capabilities,
        // `users.manage` acts as the umbrella grant.
        // API tokens of a user (service-account credentials): listing is a
        // read, issuing/revoking is an edit — `users.manage` umbrellas both.
        ("GET", ["", "api", "v1", "users", _, "tokens", ..]) => {
            vec!["user.read", "users.manage"]
        }
        (_, ["", "api", "v1", "users", _, "tokens", ..]) => {
            vec!["user.edit", "users.manage"]
        }
        ("GET", ["", "api", "v1", "users", ..]) => vec!["user.read", "users.manage"],
        ("POST", ["", "api", "v1", "users"]) => vec!["user.create", "users.manage"],
        ("PUT", ["", "api", "v1", "users", _]) => vec!["user.edit", "users.manage"],
        ("DELETE", ["", "api", "v1", "users", _]) => vec!["user.delete", "users.manage"],
        (_, ["", "api", "v1", "users", ..]) => vec!["users.manage"],

        // Requests / workflows mutations
        ("POST", ["", "api", "v1", "requests"]) => one("entity.create"),
        (_, ["", "api", "v1", "requests", ..]) => one("entity.read"),
        ("POST", ["", "api", "v1", "workflows"]) => one("entity.create"),
        ("POST", ["", "api", "v1", "workflows", _, "transition"]) => one("entity.edit"),
        (_, ["", "api", "v1", "workflows", ..]) => one("entity.read"),

        // Reports
        ("GET", ["", "api", "v1", "reports", ..]) => one("report.view"),
        ("POST", ["", "api", "v1", "reports"]) => one("report.run"),
        (_, ["", "api", "v1", "reports", _, "download"]) => one("report.export"),

        // Assessments catalog
        ("GET", ["", "api", "v1", "assessments", ..]) => one("assessment.view"),

        // Dashboard aggregates
        ("GET", ["", "api", "v1", "dashboard", ..]) => one("entity.read"),

        // Entity graph / query reads that happen to use POST
        ("POST", ["", "api", "v1", "entities", _, "graph"]) => one("entity.read"),
        ("POST", ["", "api", "v1", "entities", "query"]) => one("query.advanced"),

        // Entity assessments / risk factors
        ("POST", ["", "api", "v1", "entities", _, "assessment"]) => one("assessment.view"),
        ("GET", ["", "api", "v1", "entities", _, "assessment"]) => one("assessment.view"),
        ("POST", ["", "api", "v1", "entities", _, "factors", _, "resolve"]) => one("risk.resolve"),
        ("GET", ["", "api", "v1", "entities", _, "factors", ..]) => one("risk.view"),

        // Entity tasks and task lifecycle
        ("POST", ["", "api", "v1", "entities", _, "tasks"]) => one("entity.edit"),
        ("GET", ["", "api", "v1", "entities", _, "tasks"]) => one("entity.read"),
        (_, ["", "api", "v1", "tasks", _, ..]) => one("entity.edit"),

        // Entity documents and staged file uploads
        ("GET", ["", "api", "v1", "entities", _, "documents", ..]) => one("entity.read"),
        (_, ["", "api", "v1", "entities", _, "documents", ..]) => one("entity.edit"),
        (_, ["", "api", "v1", "files", ..]) => one("entity.edit"),

        // Entity relations
        ("GET", ["", "api", "v1", "entities", _, "relations", ..]) => one("entity.read"),
        (_, ["", "api", "v1", "entities", _, "relations", ..]) => one("entity.edit"),

        // Entity lifecycle
        ("POST", ["", "api", "v1", "entities"]) => one("entity.create"),
        ("PUT", ["", "api", "v1", "entities", _]) => one("entity.edit"),
        ("POST", ["", "api", "v1", "entities", _, "retire"]) => one("entity.retire"),
        ("POST", ["", "api", "v1", "entities", _, "merge"]) => one("entity.merge"),
        ("GET", ["", "api", "v1", "entities", ..]) => one("entity.read"),

        // Monitors
        (_, ["", "api", "v1", "monitors", ..]) => one("monitor.manage"),

        _ => vec![],
    }
}

/// API envelope wrapper.
#[derive(Debug, Serialize)]
pub struct ApiEnvelope<T: Serialize> {
    pub data: T,
    pub meta: serde_json::Value,
}

pub(crate) fn meta() -> serde_json::Value {
    serde_json::json!({ "request_id": uuid::Uuid::now_v7().to_string() })
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TypeFieldRecord {
    pub alias: String,
    pub label: String,
    pub kind: String,
    pub required: bool,
    pub identity: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub target_type_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SchemaTypeRecord {
    pub id: String,
    pub name: String,
    pub display_name: String,
    pub status: String,
    pub version: i32,
    #[serde(default)]
    pub description: String,
    #[serde(default = "default_entity_class")]
    pub entity_class: String,
    #[serde(default)]
    pub entity_count: usize,
    #[serde(default)]
    pub fields: Vec<TypeFieldRecord>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct EntityRecord {
    pub public_id: String,
    pub type_id: String,
    pub entity_class: String,
    pub revision: i64,
    pub display_name: String,
    pub status: String,
    pub created_at: String,
    pub updated_at: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fields: Option<serde_json::Value>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct QueryResponseRecord {
    pub rows: Vec<serde_json::Value>,
    pub cursor: Option<String>,
    pub total_estimate: Option<i64>,
    /// True when the candidate fetch hit its internal window cap — `rows`,
    /// `total_estimate` and `cursor` then cover only that window, not the
    /// full tenant dataset. Clients should narrow `from`/`where` instead of
    /// paginating through a truncated window.
    #[serde(default)]
    pub truncated: bool,
}

/// One preview field on a relation target card. `value` is the canonical
/// wire value of the scalar statement (string, number, object, ...).
/// Order follows the type's `preview` list in the published schema.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct RelationPreviewField {
    pub field: String,
    pub value: serde_json::Value,
}

/// An edge of a trans-object (junction entity) other than the edge that
/// brought us to it. Rendered by the UI as the "through" part of a
/// composite relation (e.g. ФЛ → Должность → {ЮЛ, рабочий телефон}).
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct RelationThroughEdge {
    pub relation_id: String,
    pub predicate: String,
    /// Direction relative to the trans-object ("outgoing"/"incoming").
    pub direction: String,
    pub target_entity_id: String,
    pub target_display_name: String,
    pub target_type: String,
    #[serde(default)]
    pub preview: Vec<RelationPreviewField>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct RelationRecord {
    pub relation_id: String,
    pub predicate: String,
    pub direction: String,
    pub target_entity_id: String,
    pub target_display_name: String,
    pub target_type: String,
    pub valid_from: String,
    pub valid_to: Option<String>,
    pub source: String,
    pub qualifiers: serde_json::Value,
    /// Relation group id from the predicate's `group` attribute in the
    /// published schema ("employment", "contacts", ...); "general" when
    /// the predicate declares none.
    #[serde(default)]
    pub group: String,
    /// Ordered preview fields of the target entity (the type's `preview`
    /// view), permission-filtered and masked like regular fields.
    #[serde(default)]
    pub preview: Vec<RelationPreviewField>,
    /// True when the target is a trans-object: its own type declares
    /// `"trans": true` in the published schema and the object exists to
    /// carry relation attributes plus onward edges.
    #[serde(default)]
    pub trans: bool,
    /// The trans-object's other edges (one hop). Empty for regular targets.
    #[serde(default)]
    pub through: Vec<RelationThroughEdge>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct HistoryEntryRecord {
    pub revision: i64,
    pub timestamp: String,
    pub actor: String,
    pub operation: String,
    pub reason: Option<String>,
    pub changes: Vec<HistoryChangeRecord>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct HistoryChangeRecord {
    pub predicate_id: String,
    pub alias: String,
    pub old_value: Option<String>,
    pub new_value: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphNodeRecord {
    pub id: String,
    pub label: String,
    pub type_label: String,
    pub type_id: String,
    pub status: String,
    pub has_children: bool,
    pub cycle: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphEdgeRecord {
    pub id: String,
    pub source: String,
    pub target: String,
    pub predicate: String,
    pub label: String,
    pub cycle: bool,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct GraphDataRecord {
    pub nodes: Vec<GraphNodeRecord>,
    pub edges: Vec<GraphEdgeRecord>,
}

fn view_field(field: &TypeFieldRecord, order: usize) -> Value {
    serde_json::json!({
        "predicate_id": field.alias,
        "alias": field.alias,
        "label_key": field.label,
        "kind": field.kind,
        "required": field.required,
        "read_only": false,
        "identity": field.identity,
        "hidden": false,
        "min_cardinality": if field.required { 1 } else { 0 },
        "max_cardinality": 1,
        "order": order,
        "target_type_id": field.target_type_id,
    })
}

pub(crate) fn build_type_view(schema_type: &SchemaTypeRecord, view_type: &str) -> Value {
    let fields = schema_type
        .fields
        .iter()
        .enumerate()
        .map(|(order, field)| view_field(field, order))
        .collect::<Vec<_>>();
    let mut columns = vec![serde_json::json!({
        "predicate_id": "display_name",
        "alias": "display_name",
        "label_key": "fields.common.name",
        "kind": "text",
        "required": false,
        "read_only": true,
        "identity": false,
        "hidden": false,
        "min_cardinality": 0,
        "max_cardinality": 1,
        "order": 0,
    })];
    columns.extend(
        schema_type
            .fields
            .iter()
            .filter(|field| field.alias != "display_name")
            .take(3)
            .enumerate()
            .map(|(order, field)| view_field(field, order + 1)),
    );
    serde_json::json!({
        "view_id": format!("{}_{}", schema_type.id, view_type),
        "view_type": view_type,
        "type_id": schema_type.id,
        "schema_revision": schema_type.version,
        "name_key": format!("types.{}", schema_type.id),
        "sections": [{
            "label_key": "sections.main",
            "order": 0,
            "collapsible": false,
            "default_collapsed": false,
            "fields": fields,
        }],
        "columns": columns,
        "default_sort": [{ "field": "display_name", "direction": "asc" }],
        "filters": columns,
        "facets": [],
        "row_actions": [],
        "bulk_actions": [],
        "export_allowed": true,
    })
}
fn default_entity_class() -> String {
    "domain".to_string()
}
