//! Field kinds: canonical scalar kinds (docs/03 §7) plus the presentation
//! kinds of the metadata-driven form registry (docs/16 §6).
//!
//! Two layers use this module:
//!
//! - `schema_admin_routes::validate_record` rejects unknown `fields[].kind`
//!   values on draft validate/publish (`field.kind.unknown`) and warns when a
//!   declarable constraint is missing (enum without `values`, entity
//!   reference without `target_type_id`).
//! - `entity_pg` write paths coerce `fields`-shorthand / kindless statement
//!   values into the explicit `{kind, value, ...}` input understood by
//!   `input_to_object`, honouring the kind declared on the published type.

use serde_json::{json, Value};

/// Field kinds a type definition may declare. Everything not listed is
/// rejected by draft validation; on the write path unknown kinds fall back
/// to `text` (lenient reads, strict schema).
pub(crate) const KNOWN_FIELD_KINDS: &[&str] = &[
    // canonical scalars (docs/03 §7)
    "text",
    "localized_text",
    "ltext",
    "boolean",
    "integer",
    "decimal",
    "date",
    "time",
    "instant",
    "datetime",
    "local_datetime",
    "timezone",
    "duration",
    "percent",
    "uuid",
    "uri",
    "url",
    "email",
    "phone",
    "money",
    "measurement",
    "geo",
    "json",
    "blob",
    "entity",
    // presentation kinds (docs/16 §6)
    "textarea",
    "rich_text",
    "enum",
    "dictionary",
    "entity_reference",
    "multi_value",
    "repeating_group",
    "tags",
    "address",
    "contact",
    "document",
    "file",
    "derived",
    "relation_editor",
];

/// Field kinds that store an entity object rather than a scalar.
const ENTITY_KINDS: &[&str] = &["entity", "entity_reference", "relation_editor"];

/// Kinds whose canonical input carries modifiers next to `value`
/// (`unit`, `currency`, `zone`, `language_tag`, `content_hash`, ...). For
/// these a kindless `{value, unit}` object is injected with the declared
/// kind as a whole, not nested under `value`.
const STRUCTURED_INPUT_KINDS: &[&str] = &[
    "money",
    "measurement",
    "local_datetime",
    "datetime",
    "instant",
    "email",
    "phone",
    "blob",
    "file",
    "document",
    "localized_text",
    "ltext",
];

/// Declared kind of a type field entry (`kind` or `scalar_kind`).
fn explicit_kind(field: &Value) -> Option<&str> {
    field
        .get("kind")
        .or_else(|| field.get("scalar_kind"))
        .and_then(Value::as_str)
}

/// True when the input already carries an explicit canonical form — the
/// write path must not reinterpret it.
fn is_explicit_input(input: &Value) -> bool {
    input.get("kind").is_some()
}

/// Extract the enum/dictionary candidate from a raw or `{value: x}` input.
fn enum_candidate(input: &Value) -> Result<String, String> {
    match input {
        Value::String(s) => Ok(s.clone()),
        Value::Number(_) | Value::Bool(_) => Ok(input.to_string()),
        _ => Err("enum value must be a string".to_string()),
    }
}

/// Validate `raw` against a field's `values`/`options` allowlist, if one is
/// declared. Returns the stringified value for storage.
fn enum_checked(field: Option<&Value>, raw: String) -> Result<String, String> {
    if let Some(values) = field
        .and_then(|f| f.get("values").or_else(|| f.get("options")))
        .and_then(Value::as_array)
    {
        let allowed: Vec<String> = values
            .iter()
            .filter_map(|v| {
                v.as_str()
                    .map(str::to_string)
                    .or_else(|| v.get("value").and_then(Value::as_str).map(str::to_string))
            })
            .collect();
        if !allowed.is_empty() && !allowed.iter().any(|a| a == &raw) {
            return Err(format!(
                "value `{raw}` is not in the allowed list [{}]",
                allowed.join(", ")
            ));
        }
    }
    Ok(raw)
}

/// Index a type definition's `fields`/`predicates` array by alias so the
/// write paths can look the declared kind up per statement alias.
pub(crate) fn field_map_of(type_def: Option<&Value>) -> std::collections::HashMap<String, Value> {
    type_def
        .and_then(|t| t.get("fields").or_else(|| t.get("predicates")))
        .and_then(Value::as_array)
        .map(|fields| {
            fields
                .iter()
                .filter_map(|f| {
                    let alias = f
                        .get("alias")
                        .or_else(|| f.get("predicate_id"))
                        .or_else(|| f.get("stable_alias"))
                        .and_then(Value::as_str)?;
                    Some((alias.to_string(), f.clone()))
                })
                .collect()
        })
        .unwrap_or_default()
}

/// Translate a raw `fields`-shorthand / kindless statement value into the
/// explicit `{kind, value, ...}` input consumed by `input_to_object`.
///
/// `field` is the type's field definition for this alias (may be `None` —
/// then everything falls back to `text`). Errors are returned as user-facing
/// strings and surfaced as HTTP 422 by the caller.
pub(crate) fn coerce_field_input(field: Option<&Value>, input: &Value) -> Result<Value, String> {
    let declared = field.and_then(explicit_kind);
    if is_explicit_input(input) {
        // An explicit `{kind, ...}` form is passed through untouched — but it
        // still must not contradict the field it lands on: a scalar kind on
        // an entity-typed field (or vice versa), a value outside a declared
        // enum allowlist, or a write to a `derived` field are rejected.
        let in_kind = input
            .get("kind")
            .and_then(Value::as_str)
            .unwrap_or("text")
            .to_ascii_lowercase();
        match declared {
            Some("derived") => {
                return Err("derived fields are computed and cannot be written".to_string());
            }
            Some(k) if ENTITY_KINDS.contains(&k) && !ENTITY_KINDS.contains(&in_kind.as_str()) => {
                return Err(format!(
                    "field expects an entity reference, got kind `{in_kind}`"
                ));
            }
            Some(k) if !ENTITY_KINDS.contains(&k) && ENTITY_KINDS.contains(&in_kind.as_str()) => {
                return Err(format!("field expects kind `{k}`, got an entity reference"));
            }
            Some("enum" | "dictionary") => {
                let raw = enum_candidate(input.get("value").unwrap_or(&Value::Null))?;
                enum_checked(field, raw)?;
            }
            _ => {}
        }
        return Ok(input.clone());
    }
    // `{entity_id}` / `{entity_public_id}` objects are entity references —
    // but only on fields that are (or may be) relation-typed. On a scalar-
    // declared field the same object is ordinary data (`json` fields can
    // legitimately hold `{"entity_id": 5}`).
    if input.get("entity_id").is_some() || input.get("entity_public_id").is_some() {
        if declared.is_none() || declared.is_some_and(|k| ENTITY_KINDS.contains(&k)) {
            let target = input
                .get("entity_id")
                .or_else(|| input.get("entity_public_id"))
                .and_then(Value::as_str);
            if target.is_none() {
                return Err("entity_reference requires an entity id".to_string());
            }
            let mut o = input.clone();
            o.as_object_mut()
                .expect("checked object")
                .insert("kind".to_string(), Value::from("entity"));
            return Ok(o);
        }
    }
    let kind = declared.unwrap_or("text");
    // Structured kinds take `{value, unit|currency|zone|...}` modifiers at
    // the top level — inject the declared kind into the object itself.
    if STRUCTURED_INPUT_KINDS.contains(&kind) && input.is_object() {
        let mut o = input.clone();
        o.as_object_mut()
            .expect("checked object")
            .insert("kind".to_string(), Value::from(kind));
        return Ok(o);
    }
    // A bare `{"value": x}` wrapper is unwrapped — it is how clients ship
    // kindless raw values (e.g. the MCP `entities.update` fields shim).
    if let Some(o) = input.as_object() {
        if o.len() == 1 && o.contains_key("value") {
            return coerce_field_input(field, &o["value"]);
        }
    }
    match kind {
        k if ENTITY_KINDS.contains(&k) => {
            let target = input
                .as_str()
                .or_else(|| {
                    input
                        .get("entity_id")
                        .or_else(|| input.get("entity_public_id"))
                        .or_else(|| input.get("value"))
                        .and_then(Value::as_str)
                })
                .ok_or_else(|| "entity_reference requires an entity id".to_string())?;
            Ok(json!({ "kind": "entity", "entity_public_id": target }))
        }
        "enum" | "dictionary" => {
            let raw = enum_checked(field, enum_candidate(input)?)?;
            Ok(json!({ "kind": kind, "value": raw }))
        }
        "money" => {
            let amount = input
                .get("amount")
                .or_else(|| input.get("value"))
                .cloned()
                .unwrap_or_else(|| input.clone());
            let currency = input
                .get("currency")
                .and_then(Value::as_str)
                .or_else(|| {
                    field
                        .and_then(|f| f.get("currency").or_else(|| f.get("unit")))
                        .and_then(Value::as_str)
                })
                .unwrap_or("RUB");
            Ok(json!({ "kind": "money", "amount": amount, "currency": currency }))
        }
        "measurement" => {
            let amount = input
                .get("amount")
                .or_else(|| input.get("value"))
                .cloned()
                .unwrap_or_else(|| input.clone());
            let unit = input
                .get("unit")
                .and_then(Value::as_str)
                .or_else(|| field.and_then(|f| f.get("unit")).and_then(Value::as_str))
                .ok_or_else(|| "measurement requires a unit".to_string())?;
            Ok(json!({ "kind": "measurement", "amount": amount, "unit": unit }))
        }
        "derived" => Err("derived fields are computed and cannot be written".to_string()),
        "tags" | "multi_value" | "repeating_group" => {
            // A bare comma-separated string is normalised to an array;
            // arrays pass through untouched.
            let arr = match input {
                Value::String(s) => Value::Array(
                    s.split(',')
                        .map(|t| t.trim())
                        .filter(|t| !t.is_empty())
                        .map(|t| Value::String(t.to_string()))
                        .collect(),
                ),
                other => other.clone(),
            };
            Ok(json!({ "kind": kind, "value": arr }))
        }
        _ => Ok(json!({ "kind": kind, "value": input })),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn f(kind: &str) -> Value {
        json!({ "alias": "f", "kind": kind })
    }

    #[test]
    fn kindless_scalar_gets_declared_kind() {
        let out = coerce_field_input(Some(&f("decimal")), &json!(450000)).unwrap();
        assert_eq!(out, json!({ "kind": "decimal", "value": 450000 }));
        let out = coerce_field_input(Some(&f("date")), &json!("2026-10-20")).unwrap();
        assert_eq!(out, json!({ "kind": "date", "value": "2026-10-20" }));
    }

    #[test]
    fn missing_field_defaults_to_text() {
        let out = coerce_field_input(None, &json!("abc")).unwrap();
        assert_eq!(out, json!({ "kind": "text", "value": "abc" }));
    }

    #[test]
    fn explicit_kind_passes_through() {
        let input = json!({ "kind": "money", "amount": "12.5", "currency": "USD" });
        assert_eq!(coerce_field_input(Some(&f("text")), &input).unwrap(), input);
    }

    #[test]
    fn entity_id_object_becomes_entity_kind() {
        // On an entity-typed or undeclared field the `{entity_id}` object is
        // a reference; on a scalar-declared field it is ordinary data.
        let out = coerce_field_input(Some(&f("entity_reference")), &json!({ "entity_id": "abc" }))
            .unwrap();
        assert_eq!(out["kind"], "entity");
        assert_eq!(out["entity_id"], "abc");
        let out = coerce_field_input(None, &json!({ "entity_id": "abc" })).unwrap();
        assert_eq!(out["kind"], "entity");
        let out = coerce_field_input(Some(&f("json")), &json!({ "entity_id": 5 })).unwrap();
        assert_eq!(out, json!({ "kind": "json", "value": { "entity_id": 5 } }));
        assert!(
            coerce_field_input(Some(&f("entity_reference")), &json!({ "entity_id": null }))
                .is_err()
        );
    }

    #[test]
    fn explicit_input_still_respects_declaration() {
        // scalar kind on an entity-typed field → reject
        assert!(coerce_field_input(
            Some(&f("entity_reference")),
            &json!({ "kind": "integer", "value": 5 })
        )
        .is_err());
        // entity kind on a scalar-typed field → reject
        assert!(coerce_field_input(
            Some(&f("money")),
            &json!({ "kind": "entity", "entity_id": "abc" })
        )
        .is_err());
        // explicit value still validated against a declared enum allowlist
        let field = json!({ "kind": "enum", "values": ["open", "closed"] });
        assert!(
            coerce_field_input(Some(&field), &json!({ "kind": "enum", "value": "bogus" })).is_err()
        );
        assert!(
            coerce_field_input(Some(&field), &json!({ "kind": "enum", "value": "open" })).is_ok()
        );
        // derived is never writable
        assert!(coerce_field_input(
            Some(&f("derived")),
            &json!({ "kind": "text", "value": "x" })
        )
        .is_err());
    }

    #[test]
    fn structured_objects_get_declared_kind() {
        let out = coerce_field_input(
            Some(&f("measurement")),
            &json!({ "value": 12, "unit": "kg" }),
        )
        .unwrap();
        assert_eq!(
            out,
            json!({ "kind": "measurement", "value": 12, "unit": "kg" })
        );
        let out = coerce_field_input(
            Some(&f("email")),
            &json!({ "local": "a", "domain": "b.ru" }),
        )
        .unwrap();
        assert_eq!(out["kind"], "email");
    }

    #[test]
    fn entity_reference_string_coerces() {
        let out = coerce_field_input(Some(&f("entity_reference")), &json!("01a00000-…")).unwrap();
        assert_eq!(
            out,
            json!({ "kind": "entity", "entity_public_id": "01a00000-…" })
        );
    }

    #[test]
    fn enum_rejects_unlisted_value() {
        let field = json!({ "kind": "enum", "values": ["open", "closed"] });
        assert!(coerce_field_input(Some(&field), &json!("open")).is_ok());
        let err = coerce_field_input(Some(&field), &json!("bogus")).unwrap_err();
        assert!(err.contains("allowed list"));
    }

    #[test]
    fn tags_split_comma_string() {
        let out = coerce_field_input(Some(&f("tags")), &json!("a, b ,c")).unwrap();
        assert_eq!(out, json!({ "kind": "tags", "value": ["a", "b", "c"] }));
        let out = coerce_field_input(Some(&f("multi_value")), &json!(["x"])).unwrap();
        assert_eq!(out, json!({ "kind": "multi_value", "value": ["x"] }));
    }

    #[test]
    fn money_defaults_currency_and_reads_field() {
        let out = coerce_field_input(Some(&f("money")), &json!(150000)).unwrap();
        assert_eq!(
            out,
            json!({ "kind": "money", "amount": 150000, "currency": "RUB" })
        );
        let field = json!({ "kind": "money", "currency": "USD" });
        let out = coerce_field_input(Some(&field), &json!("12.5")).unwrap();
        assert_eq!(out["currency"], "USD");
    }

    #[test]
    fn measurement_needs_unit() {
        let err = coerce_field_input(Some(&f("measurement")), &json!(12)).unwrap_err();
        assert!(err.contains("unit"));
        let field = json!({ "kind": "measurement", "unit": "kg" });
        let out = coerce_field_input(Some(&field), &json!(12)).unwrap();
        assert_eq!(out["unit"], "kg");
    }

    #[test]
    fn value_wrapper_unwraps_before_coercion() {
        // `{"value": x}` shorthand form — common in MCP/tool payloads.
        let out = coerce_field_input(Some(&f("tags")), &json!({"value": "a,b"})).unwrap();
        assert_eq!(out["value"], json!(["a", "b"]));
        let out = coerce_field_input(Some(&f("integer")), &json!({"value": 42})).unwrap();
        assert_eq!(out, json!({ "kind": "integer", "value": 42 }));
    }

    #[test]
    fn enum_accepts_numbers_and_rejects_objects() {
        let field = json!({ "kind": "enum", "values": ["1", "2"] });
        let out = coerce_field_input(Some(&field), &json!(2)).unwrap();
        assert_eq!(out["value"], "2");
        let field = json!({ "kind": "dictionary", "values": ["x"] });
        assert!(coerce_field_input(Some(&field), &json!({"a": 1})).is_err());
    }

    #[test]
    fn geo_and_json_passthrough() {
        let out = coerce_field_input(
            Some(&f("geo")),
            &json!({"type": "Point", "coordinates": [1.0, 2.0]}),
        )
        .unwrap();
        assert_eq!(out["kind"], "geo");
        let out = coerce_field_input(Some(&f("json")), &json!({"k": [1]})).unwrap();
        assert_eq!(out, json!({ "kind": "json", "value": {"k": [1]} }));
    }

    #[test]
    fn derived_rejected_before_write() {
        assert!(coerce_field_input(Some(&f("derived")), &json!("x")).is_err());
    }

    #[test]
    fn datetime_local_and_instant_kinds() {
        let out =
            coerce_field_input(Some(&f("local_datetime")), &json!("2026-10-20T10:00")).unwrap();
        assert_eq!(out["kind"], "local_datetime");
        let out = coerce_field_input(Some(&f("instant")), &json!("2026-10-20T10:00:00Z")).unwrap();
        assert_eq!(out["kind"], "instant");
    }
}
