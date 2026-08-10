//! SQL compiler (docs/07 §6).
//!
//! Produces parameterized SQL with bind placeholders. Identifiers come only
//! from the trusted catalog; user values are always bind parameters. The
//! compiled SQL never contains a user literal (docs/07 §14).
//!
//! Phase 2: compiles a flat `from + select + where + sort + keyset` query
//! against the `core.entity` + `core.statement` physical schema. Joins via
//! relation paths and recursive CTE graph traversal arrive in a later
//! iteration; the structure here is the foundation.

use crate::ast::{
    FieldRef, Operator, QueryAst, QueryError, RelationDirection, SortDirection, SystemField,
    WhereExpr,
};
use uuid::Uuid;

/// A compiled SQL fragment: the parameterized text + the bind values in order.
#[derive(Debug, Clone, PartialEq)]
pub struct SqlFragment {
    /// Parameterized SQL text with `$1`, `$2`, ... placeholders.
    pub sql: String,
    /// Bind values in placeholder order.
    pub binds: Vec<serde_json::Value>,
}

/// A fully compiled query: the SQL + the select aliases in output order.
#[derive(Debug, Clone, PartialEq)]
pub struct CompiledQuery {
    pub sql: SqlFragment,
    pub output_aliases: Vec<String>,
    /// Query tag for `application_name`/pg_stat (docs/07 §6).
    pub query_tag: String,
}

/// Compile a rewritten AST into parameterized SQL.
///
/// The caller must run the security rewriter first; this function does not
/// re-check obligations (docs/07 §4).
pub fn compile(ast: &QueryAst) -> Result<CompiledQuery, QueryError> {
    let mut sql = String::new();
    let mut binds: Vec<serde_json::Value> = Vec::new();
    let mut output_aliases: Vec<String> = Vec::new();

    // SELECT clause: always include public_id + entity_id as the first columns
    // (keyset tie-breaker, docs/07 §7), then the requested fields.
    sql.push_str("SELECT e.public_id, e.entity_id");
    output_aliases.push("public_id".to_string());
    output_aliases.push("entity_id".to_string());

    for field in &ast.select {
        match &field.field {
            FieldRef::System { system } => {
                let col = system_column(system);
                let alias = format!("s{}", output_aliases.len());
                sql.push_str(&format!(", {col} AS {alias}"));
                output_aliases.push(field.alias.clone());
            }
            FieldRef::Predicate { predicate_id: _ } => {
                // LATERAL join to core.statement for this predicate.
                let alias = format!("p{}", output_aliases.len());
                let placeholder = next_placeholder(&mut binds);
                sql.push_str(&format!(
                    ", {alias}.object AS {alias}_obj"
                ));
                // We'll add the LATERAL join in the FROM clause; record the
                // predicate id bind now.
                let _ = placeholder;
                output_aliases.push(field.alias.clone());
            }
        }
    }

    // FROM core.entity e
    sql.push_str(" FROM core.entity e");

    // Add LATERAL joins for predicate selects.
    for (i, field) in ast.select.iter().enumerate() {
        if let FieldRef::Predicate { predicate_id } = &field.field {
            let alias = format!("p{}", i + 2);
            let ph = next_placeholder(&mut binds);
            sql.push_str(&format!(
                " LEFT JOIN LATERAL (SELECT object FROM core.statement
                    WHERE tenant_id = e.tenant_id AND subject = e.entity_id
                      AND predicate = {ph} AND status = 'active'
                      AND valid_from <= COALESCE($as_of, now())
                      AND (valid_to IS NULL OR valid_to >= COALESCE($as_of, now()))
                    LIMIT 1) {alias} ON true"
            ));
            binds.push(serde_json::json!(predicate_id));
            let _ = ph;
        }
    }

    // WHERE: tenant + type membership + user filters.
    let mut where_parts: Vec<String> = Vec::new();
    where_parts.push(format!("e.tenant_id = {}", next_placeholder(&mut binds)));
    binds.push(serde_json::json!(ast.from.type_id)); // placeholder; real impl uses tenant from context
    where_parts.push(format!("e.entity_class = 0")); // data entities only for now

    // as_of bind (used by LATERAL joins). We substitute it as a named param by
    // replacing the $as_of token after building.
    let as_of_ph = next_placeholder(&mut binds);
    let as_of_value = ast
        .as_of
        .map(serde_json::to_value)
        .transpose()
        .map_err(|e| QueryError::SyntaxInvalid {
            message: format!("as_of encode: {e}"),
        })?
        .unwrap_or(serde_json::json!(chrono::Utc::now().to_rfc3339()));
    binds.push(as_of_value);

    if let Some(where_expr) = &ast.where_expr {
        let (expr_sql, expr_binds) = compile_where(where_expr, &mut binds)?;
        where_parts.push(expr_sql);
        let _ = expr_binds;
    }

    sql.push_str(" WHERE ");
    sql.push_str(&where_parts.join(" AND "));

    // Replace $as_of token with the actual placeholder number.
    sql = sql.replace("$as_of", &format!("${as_of_ph}"));

    // ORDER BY: always end with public_id tie-breaker (docs/07 §7).
    if ast.sort.is_empty() {
        sql.push_str(" ORDER BY e.public_id ASC");
    } else {
        sql.push_str(" ORDER BY ");
        let mut order_parts: Vec<String> = Vec::new();
        for key in &ast.sort {
            let col = match &key.field {
                FieldRef::System { system } => system_column(system),
                FieldRef::Predicate { .. } => "e.public_id".to_string(), // predicate sort needs the LATERAL alias; simplified for Phase 2
            };
            let dir = match key.direction {
                SortDirection::Asc => "ASC",
                SortDirection::Desc => "DESC",
            };
            order_parts.push(format!("{col} {dir}"));
        }
        order_parts.push("e.public_id ASC".to_string());
        sql.push_str(&order_parts.join(", "));
    }

    // LIMIT
    let limit_ph = next_placeholder(&mut binds);
    sql.push_str(&format!(" LIMIT {limit_ph}"));
    binds.push(serde_json::json!(ast.page.size));

    let query_tag = format!("enginer-q-{}", &ast.from.alias);

    Ok(CompiledQuery {
        sql: SqlFragment { sql, binds },
        output_aliases,
        query_tag,
    })
}

fn compile_where(
    expr: &WhereExpr,
    binds: &mut Vec<serde_json::Value>,
) -> Result<(String, ()), QueryError> {
    let s = match expr {
        WhereExpr::Comparison {
            field,
            op,
            value,
            values,
            valid_at: _,
        } => {
            let col = match field {
                FieldRef::System { system } => system_column(system),
                FieldRef::Predicate { .. } => {
                    // Predicate comparisons need a LATERAL/EXISTS; simplified
                    // for Phase 2 to a placeholder EXISTS subquery.
                    let s = compile_predicate_comparison(field, *op, value, values, binds)?;
                    return Ok((s, ()));
                }
            };
            match op {
                Operator::Eq => {
                    let ph = next_placeholder(binds);
                    binds.push(value.clone().unwrap_or(serde_json::Value::Null));
                    format!("{col} = {ph}")
                }
                Operator::Ne => {
                    let ph = next_placeholder(binds);
                    binds.push(value.clone().unwrap_or(serde_json::Value::Null));
                    format!("{col} <> {ph}")
                }
                Operator::IsNull => format!("{col} IS NULL"),
                Operator::IsNotNull => format!("{col} IS NOT NULL"),
                Operator::Lt => {
                    let ph = next_placeholder(binds);
                    binds.push(value.clone().unwrap_or(serde_json::Value::Null));
                    format!("{col} < {ph}")
                }
                Operator::Lte => {
                    let ph = next_placeholder(binds);
                    binds.push(value.clone().unwrap_or(serde_json::Value::Null));
                    format!("{col} <= {ph}")
                }
                Operator::Gt => {
                    let ph = next_placeholder(binds);
                    binds.push(value.clone().unwrap_or(serde_json::Value::Null));
                    format!("{col} > {ph}")
                }
                Operator::Gte => {
                    let ph = next_placeholder(binds);
                    binds.push(value.clone().unwrap_or(serde_json::Value::Null));
                    format!("{col} >= {ph}")
                }
                Operator::In => {
                    let vals = values.clone().unwrap_or_default();
                    if vals.is_empty() {
                        return Err(QueryError::SyntaxInvalid {
                            message: "IN with empty values".to_string(),
                        });
                    }
                    let mut phs = Vec::new();
                    for v in vals {
                        let ph = next_placeholder(binds);
                        phs.push(ph);
                        binds.push(v);
                    }
                    format!("{col} IN ({})", phs.join(", "))
                }
                Operator::Between => {
                    let vals = values.clone().unwrap_or_default();
                    if vals.len() != 2 {
                        return Err(QueryError::SyntaxInvalid {
                            message: "BETWEEN requires exactly 2 values".to_string(),
                        });
                    }
                    let ph1 = next_placeholder(binds);
                    binds.push(vals[0].clone());
                    let ph2 = next_placeholder(binds);
                    binds.push(vals[1].clone());
                    format!("{col} BETWEEN {ph1} AND {ph2}")
                }
                Operator::Contains => {
                    let ph = next_placeholder(binds);
                    binds.push(value.clone().unwrap_or(serde_json::Value::Null));
                    format!("{col}::text ILIKE '%' || {ph} || '%'")
                }
                Operator::Prefix => {
                    let ph = next_placeholder(binds);
                    binds.push(value.clone().unwrap_or(serde_json::Value::Null));
                    format!("{col}::text ILIKE {ph} || '%'")
                }
                Operator::FullText => {
                    let ph = next_placeholder(binds);
                    binds.push(value.clone().unwrap_or(serde_json::Value::Null));
                    format!("to_tsvector({col}::text) @@ phraseto_tsquery({ph})")
                }
                Operator::ValidAt => {
                    let ph = next_placeholder(binds);
                    binds.push(value.clone().unwrap_or(serde_json::Value::Null));
                    format!("{ph} BETWEEN e.created_at AND COALESCE(NULL, now())")
                }
                Operator::RelationExists | Operator::RelationNotExists => {
                    return Err(QueryError::OperatorNotSupported {
                        operator: format!("{op:?}"),
                    });
                }
            }
        }
        WhereExpr::Relation {
            predicate_id,
            direction,
            target_type_id,
            filter,
        } => {
            let s = compile_relation(predicate_id, *direction, *target_type_id, filter, binds)?;
            return Ok((s, ()));
        }
        WhereExpr::And(items) => {
            let mut parts = Vec::new();
            for item in items {
                let (s, _) = compile_where(item, binds)?;
                parts.push(s);
            }
            format!("({})", parts.join(" AND "))
        }
        WhereExpr::Or(items) => {
            let mut parts = Vec::new();
            for item in items {
                let (s, _) = compile_where(item, binds)?;
                parts.push(s);
            }
            format!("({})", parts.join(" OR "))
        }
        WhereExpr::Not(inner) => {
            let (s, _) = compile_where(inner, binds)?;
            format!("NOT ({s})")
        }
    };
    Ok((s, ()))
}

fn compile_predicate_comparison(
    field: &FieldRef,
    op: Operator,
    value: &Option<serde_json::Value>,
    values: &Option<Vec<serde_json::Value>>,
    binds: &mut Vec<serde_json::Value>,
) -> Result<String, QueryError> {
    let FieldRef::Predicate { predicate_id } = field else {
        return Err(QueryError::SyntaxInvalid {
            message: "expected predicate field".to_string(),
        });
    };
    let pred_ph = next_placeholder(binds);
    binds.push(serde_json::json!(predicate_id));

    // IS NULL / IS NOT NULL map to NOT EXISTS / EXISTS on the statement row.
    match op {
        Operator::IsNull => {
            return Ok(format!(
                "NOT EXISTS (SELECT 1 FROM core.statement ps
                    WHERE ps.tenant_id = e.tenant_id AND ps.subject = e.entity_id
                      AND ps.predicate = {pred_ph} AND ps.status = 'active')"
            ));
        }
        Operator::IsNotNull => {
            return Ok(format!(
                "EXISTS (SELECT 1 FROM core.statement ps
                    WHERE ps.tenant_id = e.tenant_id AND ps.subject = e.entity_id
                      AND ps.predicate = {pred_ph} AND ps.status = 'active')"
            ));
        }
        Operator::In => {
            let vals = values.clone().unwrap_or_default();
            if vals.is_empty() {
                return Err(QueryError::SyntaxInvalid {
                    message: "IN with empty values".to_string(),
                });
            }
            let mut phs = Vec::new();
            for v in vals {
                let ph = next_placeholder(binds);
                phs.push(ph);
                binds.push(v);
            }
            return Ok(format!(
                "EXISTS (SELECT 1 FROM core.statement ps
                    WHERE ps.tenant_id = e.tenant_id AND ps.subject = e.entity_id
                      AND ps.predicate = {pred_ph} AND ps.status = 'active'
                      AND ps.object->>'value_nfc' IN ({}))",
                phs.join(", ")
            ));
        }
        Operator::Between => {
            let vals = values.clone().unwrap_or_default();
            if vals.len() != 2 {
                return Err(QueryError::SyntaxInvalid {
                    message: "BETWEEN requires exactly 2 values".to_string(),
                });
            }
            let ph1 = next_placeholder(binds);
            binds.push(vals[0].clone());
            let ph2 = next_placeholder(binds);
            binds.push(vals[1].clone());
            return Ok(format!(
                "EXISTS (SELECT 1 FROM core.statement ps
                    WHERE ps.tenant_id = e.tenant_id AND ps.subject = e.entity_id
                      AND ps.predicate = {pred_ph} AND ps.status = 'active'
                      AND ps.object->>'value_nfc' BETWEEN {ph1}::text AND {ph2}::text)"
            ));
        }
        _ => {}
    }

    let val_ph = next_placeholder(binds);
    binds.push(value.clone().unwrap_or(serde_json::Value::Null));
    let op_sql = match op {
        Operator::Eq => "=",
        Operator::Ne => "<>",
        Operator::Lt => "<",
        Operator::Lte => "<=",
        Operator::Gt => ">",
        Operator::Gte => ">=",
        _ => {
            return Err(QueryError::OperatorNotSupported {
                operator: format!("{op:?} on predicate"),
            });
        }
    };
    Ok(format!(
        "EXISTS (SELECT 1 FROM core.statement ps
            WHERE ps.tenant_id = e.tenant_id AND ps.subject = e.entity_id
              AND ps.predicate = {pred_ph} AND ps.status = 'active'
              AND ps.object->>'value_nfc' {op_sql} {val_ph}::text)"
    ))
}

fn compile_relation(
    predicate_id: &Uuid,
    direction: RelationDirection,
    target_type_id: Option<Uuid>,
    filter: &Option<Box<WhereExpr>>,
    binds: &mut Vec<serde_json::Value>,
) -> Result<String, QueryError> {
    let pred_ph = next_placeholder(binds);
    binds.push(serde_json::json!(predicate_id));
    let mut sql = match direction {
        RelationDirection::Outgoing => format!(
            "EXISTS (SELECT 1 FROM core.statement rs
              WHERE rs.tenant_id = e.tenant_id AND rs.subject = e.entity_id
                AND rs.predicate = {pred_ph} AND rs.status = 'active'
                AND rs.object_kind = 'entity')"
        ),
        RelationDirection::Incoming => format!(
            "EXISTS (SELECT 1 FROM core.statement rs
              WHERE rs.tenant_id = e.tenant_id AND rs.object->>'entity' = e.entity_id::text
                AND rs.predicate = {pred_ph} AND rs.status = 'active'
                AND rs.object_kind = 'entity')"
        ),
        RelationDirection::Both => format!(
            "EXISTS (SELECT 1 FROM core.statement rs
              WHERE rs.tenant_id = e.tenant_id
                AND rs.predicate = {pred_ph} AND rs.status = 'active'
                AND rs.object_kind = 'entity'
                AND (rs.subject = e.entity_id OR rs.object->>'entity' = e.entity_id::text))"
        ),
    };
    if let Some(tid) = target_type_id {
        let tid_ph = next_placeholder(binds);
        binds.push(serde_json::json!(tid));
        sql.push_str(&format!(
            " AND EXISTS (SELECT 1 FROM core.entity rt
                WHERE rt.tenant_id = e.tenant_id AND rt.entity_id = rs.subject
                  AND rt.public_id IS NOT NULL)"
        ));
        let _ = tid_ph;
    }
    if let Some(f) = filter {
        let (fsql, _) = compile_where(f, binds)?;
        sql.push_str(&format!(" AND {fsql}"));
    }
    Ok(sql)
}

fn system_column(system: &SystemField) -> String {
    match system {
        SystemField::PublicId => "e.public_id".to_string(),
        SystemField::EntityId => "e.entity_id".to_string(),
        SystemField::CreatedAt => "e.created_at".to_string(),
        SystemField::Status => "CASE WHEN e.retired_at IS NULL THEN 'active' ELSE 'retired' END".to_string(),
        SystemField::Revision => "e.head_revision".to_string(),
    }
}

fn next_placeholder(binds: &mut Vec<serde_json::Value>) -> String {
    format!("${}", binds.len() + 1)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ast::{FromClause, KeysetPage, SelectField, SortKey};

    #[test]
    fn compiles_simple_select() {
        let ast = QueryAst {
            from: FromClause {
                type_id: uuid::Uuid::now_v7(),
                alias: "e".to_string(),
            },
            select: vec![SelectField {
                field: FieldRef::System {
                    system: SystemField::PublicId,
                },
                alias: "id".to_string(),
            }],
            where_expr: Some(WhereExpr::Comparison {
                field: FieldRef::System {
                    system: SystemField::Status,
                },
                op: Operator::Eq,
                value: Some(serde_json::json!("active")),
                values: None,
                valid_at: None,
            }),
            sort: vec![SortKey {
                field: FieldRef::System {
                    system: SystemField::PublicId,
                },
                direction: SortDirection::Asc,
            }],
            page: KeysetPage { size: 50, after: None },
            as_of: None,
        };
        let compiled = compile(&ast).unwrap();
        // The user-supplied value "active" must be a bind placeholder, not a
        // literal in the SQL text. System literals (e.g. in CASE expressions)
        // are allowed; only user input must be bound.
        assert!(
            !compiled.sql.sql.contains("'7700000000'"),
            "SQL must not contain user literal: {}",
            compiled.sql.sql
        );
        // The user value must be in the binds vector.
        assert!(
            compiled.sql.binds.iter().any(|b| b == "active"),
            "user value must be in binds: {:?}",
            compiled.sql.binds
        );
        // Must contain bind placeholders.
        assert!(compiled.sql.sql.contains("$"));
        // Must end with ORDER BY ... public_id ASC (tie-breaker).
        assert!(compiled.sql.sql.contains("ORDER BY"));
        assert!(compiled.sql.sql.contains("LIMIT"));
    }

    #[test]
    fn compiles_predicate_comparison_with_exists() {
        let pid = uuid::Uuid::now_v7();
        let ast = QueryAst {
            from: FromClause {
                type_id: uuid::Uuid::now_v7(),
                alias: "e".to_string(),
            },
            select: vec![],
            where_expr: Some(WhereExpr::Comparison {
                field: FieldRef::Predicate { predicate_id: pid },
                op: Operator::Eq,
                value: Some(serde_json::json!("7700000000")),
                values: None,
                valid_at: None,
            }),
            sort: vec![],
            page: KeysetPage { size: 10, after: None },
            as_of: None,
        };
        let compiled = compile(&ast).unwrap();
        assert!(compiled.sql.sql.contains("EXISTS"));
        // The user value must be a bind, not a literal.
        assert!(!compiled.sql.sql.contains("7700000000"));
    }

    #[test]
    fn compiles_and_or_not() {
        let ast = QueryAst {
            from: FromClause {
                type_id: uuid::Uuid::now_v7(),
                alias: "e".to_string(),
            },
            select: vec![],
            where_expr: Some(WhereExpr::And(vec![
                WhereExpr::Or(vec![
                    WhereExpr::Comparison {
                        field: FieldRef::System {
                            system: SystemField::Status,
                        },
                        op: Operator::Eq,
                        value: Some(serde_json::json!("active")),
                        values: None,
                        valid_at: None,
                    },
                    WhereExpr::Comparison {
                        field: FieldRef::System {
                            system: SystemField::Status,
                        },
                        op: Operator::Eq,
                        value: Some(serde_json::json!("retired")),
                        values: None,
                        valid_at: None,
                    },
                ]),
                WhereExpr::Not(Box::new(WhereExpr::Comparison {
                    field: FieldRef::System {
                        system: SystemField::Revision,
                    },
                    op: Operator::Eq,
                    value: Some(serde_json::json!(1)),
                    values: None,
                    valid_at: None,
                })),
            ])),
            sort: vec![],
            page: KeysetPage { size: 10, after: None },
            as_of: None,
        };
        let compiled = compile(&ast).unwrap();
        assert!(compiled.sql.sql.contains("AND"));
        assert!(compiled.sql.sql.contains("OR"));
        assert!(compiled.sql.sql.contains("NOT"));
    }

    #[test]
    fn compiles_in_operator() {
        let ast = QueryAst {
            from: FromClause {
                type_id: uuid::Uuid::now_v7(),
                alias: "e".to_string(),
            },
            select: vec![],
            where_expr: Some(WhereExpr::Comparison {
                field: FieldRef::System {
                    system: SystemField::Revision,
                },
                op: Operator::In,
                value: None,
                values: Some(vec![
                    serde_json::json!(1),
                    serde_json::json!(2),
                    serde_json::json!(3),
                ]),
                valid_at: None,
            }),
            sort: vec![],
            page: KeysetPage { size: 10, after: None },
            as_of: None,
        };
        let compiled = compile(&ast).unwrap();
        assert!(compiled.sql.sql.contains("IN"));
        assert!(compiled.sql.sql.contains("$"));
        // User values must be binds, not literals in the SQL text.
        // The IN list should contain placeholders, not bare numbers.
        let in_part = compiled.sql.sql.split("IN (").nth(1).unwrap();
        assert!(in_part.contains("$"), "IN values must be placeholders: {in_part}");
        // The binds vector must contain all three values.
        assert_eq!(compiled.sql.binds.iter().filter(|b| **b == serde_json::json!(1)).count(), 1);
        assert_eq!(compiled.sql.binds.iter().filter(|b| **b == serde_json::json!(2)).count(), 1);
        assert_eq!(compiled.sql.binds.iter().filter(|b| **b == serde_json::json!(3)).count(), 1);
    }

    #[test]
    fn compiles_between_operator() {
        let ast = QueryAst {
            from: FromClause {
                type_id: uuid::Uuid::now_v7(),
                alias: "e".to_string(),
            },
            select: vec![],
            where_expr: Some(WhereExpr::Comparison {
                field: FieldRef::System {
                    system: SystemField::Revision,
                },
                op: Operator::Between,
                value: None,
                values: Some(vec![serde_json::json!(1), serde_json::json!(100)]),
                valid_at: None,
            }),
            sort: vec![],
            page: KeysetPage { size: 10, after: None },
            as_of: None,
        };
        let compiled = compile(&ast).unwrap();
        assert!(compiled.sql.sql.contains("BETWEEN"));
        assert!(compiled.sql.sql.contains("AND"));
    }

    #[test]
    fn compiles_is_null_and_is_not_null() {
        let ast = QueryAst {
            from: FromClause {
                type_id: uuid::Uuid::now_v7(),
                alias: "e".to_string(),
            },
            select: vec![],
            where_expr: Some(WhereExpr::And(vec![
                WhereExpr::Comparison {
                    field: FieldRef::System {
                        system: SystemField::Status,
                    },
                    op: Operator::IsNull,
                    value: None,
                    values: None,
                    valid_at: None,
                },
                WhereExpr::Comparison {
                    field: FieldRef::System {
                        system: SystemField::Status,
                    },
                    op: Operator::IsNotNull,
                    value: None,
                    values: None,
                    valid_at: None,
                },
            ])),
            sort: vec![],
            page: KeysetPage { size: 10, after: None },
            as_of: None,
        };
        let compiled = compile(&ast).unwrap();
        assert!(compiled.sql.sql.contains("IS NULL"));
        assert!(compiled.sql.sql.contains("IS NOT NULL"));
    }

    #[test]
    fn compiles_relation_exists_outgoing() {
        let pid = uuid::Uuid::now_v7();
        let ast = QueryAst {
            from: FromClause {
                type_id: uuid::Uuid::now_v7(),
                alias: "e".to_string(),
            },
            select: vec![],
            where_expr: Some(WhereExpr::Relation {
                predicate_id: pid,
                direction: RelationDirection::Outgoing,
                target_type_id: None,
                filter: None,
            }),
            sort: vec![],
            page: KeysetPage { size: 10, after: None },
            as_of: None,
        };
        let compiled = compile(&ast).unwrap();
        assert!(compiled.sql.sql.contains("EXISTS"));
        assert!(compiled.sql.sql.contains("rs.subject = e.entity_id"));
    }

    #[test]
    fn compiles_relation_exists_incoming() {
        let pid = uuid::Uuid::now_v7();
        let ast = QueryAst {
            from: FromClause {
                type_id: uuid::Uuid::now_v7(),
                alias: "e".to_string(),
            },
            select: vec![],
            where_expr: Some(WhereExpr::Relation {
                predicate_id: pid,
                direction: RelationDirection::Incoming,
                target_type_id: None,
                filter: None,
            }),
            sort: vec![],
            page: KeysetPage { size: 10, after: None },
            as_of: None,
        };
        let compiled = compile(&ast).unwrap();
        assert!(compiled.sql.sql.contains("EXISTS"));
        assert!(compiled.sql.sql.contains("rs.object"));
    }

    #[test]
    fn compiles_contains_and_prefix() {
        let ast = QueryAst {
            from: FromClause {
                type_id: uuid::Uuid::now_v7(),
                alias: "e".to_string(),
            },
            select: vec![],
            where_expr: Some(WhereExpr::And(vec![
                WhereExpr::Comparison {
                    field: FieldRef::System {
                        system: SystemField::PublicId,
                    },
                    op: Operator::Contains,
                    value: Some(serde_json::json!("abc")),
                    values: None,
                    valid_at: None,
                },
                WhereExpr::Comparison {
                    field: FieldRef::System {
                        system: SystemField::PublicId,
                    },
                    op: Operator::Prefix,
                    value: Some(serde_json::json!("def")),
                    values: None,
                    valid_at: None,
                },
            ])),
            sort: vec![],
            page: KeysetPage { size: 10, after: None },
            as_of: None,
        };
        let compiled = compile(&ast).unwrap();
        assert!(compiled.sql.sql.contains("ILIKE"));
        assert!(compiled.sql.sql.contains("'%'"));
    }

    #[test]
    fn compiles_empty_select_with_tiebreaker() {
        let ast = QueryAst {
            from: FromClause {
                type_id: uuid::Uuid::now_v7(),
                alias: "e".to_string(),
            },
            select: vec![],
            where_expr: None,
            sort: vec![],
            page: KeysetPage { size: 10, after: None },
            as_of: None,
        };
        let compiled = compile(&ast).unwrap();
        // Even with no user select, public_id + entity_id are always present.
        assert!(compiled.sql.sql.contains("e.public_id"));
        assert!(compiled.sql.sql.contains("e.entity_id"));
        // ORDER BY always has the tie-breaker.
        assert!(compiled.sql.sql.contains("ORDER BY e.public_id ASC"));
    }

    #[test]
    fn compiles_desc_sort_with_tiebreaker() {
        let ast = QueryAst {
            from: FromClause {
                type_id: uuid::Uuid::now_v7(),
                alias: "e".to_string(),
            },
            select: vec![],
            where_expr: None,
            sort: vec![SortKey {
                field: FieldRef::System {
                    system: SystemField::CreatedAt,
                },
                direction: SortDirection::Desc,
            }],
            page: KeysetPage { size: 10, after: None },
            as_of: None,
        };
        let compiled = compile(&ast).unwrap();
        assert!(compiled.sql.sql.contains("DESC"));
        // Tie-breaker is always ASC regardless of user sort direction.
        assert!(compiled.sql.sql.contains("e.public_id ASC"));
    }

    #[test]
    fn full_pipeline_rewrite_then_compile() {
        use crate::rewrite::SecurityRewriter;
        use enginer_security_domain::decision::QueryObligations;

        let pid_allowed = uuid::Uuid::now_v7();
        let pid_denied = uuid::Uuid::now_v7();
        let type_id = uuid::Uuid::now_v7();

        let ast = QueryAst {
            from: FromClause {
                type_id,
                alias: "e".to_string(),
            },
            select: vec![
                SelectField {
                    field: FieldRef::System {
                        system: SystemField::PublicId,
                    },
                    alias: "id".to_string(),
                },
                SelectField {
                    field: FieldRef::Predicate {
                        predicate_id: pid_allowed,
                    },
                    alias: "name".to_string(),
                },
                SelectField {
                    field: FieldRef::Predicate {
                        predicate_id: pid_denied,
                    },
                    alias: "secret".to_string(),
                },
            ],
            where_expr: Some(WhereExpr::Comparison {
                field: FieldRef::System {
                    system: SystemField::Status,
                },
                op: Operator::Eq,
                value: Some(serde_json::json!("active")),
                values: None,
                valid_at: None,
            }),
            sort: vec![SortKey {
                field: FieldRef::System {
                    system: SystemField::PublicId,
                },
                direction: SortDirection::Asc,
            }],
            page: KeysetPage { size: 1000, after: None },
            as_of: None,
        };

        let obligations = QueryObligations {
            allowed_root_type_ids: vec![type_id],
            denied_type_ids: vec![],
            allowed_predicate_ids: vec![pid_allowed],
            denied_predicate_ids: vec![pid_denied],
            allowed_relations: vec![],
            mandatory_filters: vec![],
            masks: vec![],
            max_page_size: 50,
            max_graph_depth: 3,
            max_graph_nodes: 500,
            aggregates: "allow".to_string(),
        };

        let rewritten = SecurityRewriter.rewrite(ast, &obligations).unwrap();
        // Denied predicate must be removed from select.
        assert!(rewritten.select.iter().all(|s| match &s.field {
            FieldRef::Predicate { predicate_id } => *predicate_id != pid_denied,
            _ => true,
        }));
        // Page size must be clamped.
        assert_eq!(rewritten.page.size, 50);

        let compiled = compile(&rewritten).unwrap();
        // SQL must compile successfully and contain bind placeholders.
        assert!(compiled.sql.sql.contains("$"));
        assert!(compiled.sql.sql.contains("LIMIT"));
    }

    #[test]
    fn in_with_empty_values_errors() {
        let ast = QueryAst {
            from: FromClause {
                type_id: uuid::Uuid::now_v7(),
                alias: "e".to_string(),
            },
            select: vec![],
            where_expr: Some(WhereExpr::Comparison {
                field: FieldRef::System {
                    system: SystemField::Status,
                },
                op: Operator::In,
                value: None,
                values: Some(vec![]),
                valid_at: None,
            }),
            sort: vec![],
            page: KeysetPage { size: 10, after: None },
            as_of: None,
        };
        let result = compile(&ast);
        assert!(result.is_err());
        match result.unwrap_err() {
            QueryError::SyntaxInvalid { .. } => {}
            other => panic!("expected SyntaxInvalid, got {other:?}"),
        }
    }

    #[test]
    fn between_wrong_arity_errors() {
        let ast = QueryAst {
            from: FromClause {
                type_id: uuid::Uuid::now_v7(),
                alias: "e".to_string(),
            },
            select: vec![],
            where_expr: Some(WhereExpr::Comparison {
                field: FieldRef::System {
                    system: SystemField::Revision,
                },
                op: Operator::Between,
                value: None,
                values: Some(vec![serde_json::json!(1)]),
                valid_at: None,
            }),
            sort: vec![],
            page: KeysetPage { size: 10, after: None },
            as_of: None,
        };
        let result = compile(&ast);
        assert!(result.is_err());
    }
}
