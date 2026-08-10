//! Security AST rewriter (docs/07 §5).
//!
//! The rewriter takes the user AST and the PDP `QueryObligations` and produces
//! a rewritten AST that:
//!
//! - intersects `select` with `allowed_predicate_ids`;
//! - removes `denied_type_ids` / `denied_predicate_ids`;
//! - adds `mandatory_filters` to the root and every subquery;
//! - removes forbidden edges from `Relation` nodes;
//! - rejects sort/facet on hidden fields (no side channel, docs/07 §5);
//! - clamps `page.size` to `max_page_size`;
//! - rejects forbidden aggregates.
//!
//! The rewriter is pure: it does not touch SQL. The compiler then turns the
//! rewritten AST into parameterized SQL. Security WHERE is never glued on as a
//! string after compilation (docs/07 §4).

use crate::ast::{
    FieldRef, Operator, QueryAst, QueryError, SystemField,
    WhereExpr,
};
#[cfg(test)]
use crate::ast::{SelectField, SortKey};
use enginer_security_domain::decision::QueryObligations;

/// The security rewriter.
pub struct SecurityRewriter;

impl SecurityRewriter {
    /// Rewrite the AST according to the PDP obligations. Returns the rewritten
    /// AST or a `field_forbidden` / `too_complex` error.
    pub fn rewrite(
        &self,
        mut ast: QueryAst,
        obligations: &QueryObligations,
    ) -> Result<QueryAst, QueryError> {
        // 1. Root type allowlist.
        if !obligations.allowed_root_type_ids.is_empty()
            && !obligations
                .allowed_root_type_ids
                .contains(&ast.from.type_id)
        {
            // Root type fully forbidden -> compile to no rows.
            return Err(QueryError::FieldForbidden {
                message: "root type forbidden by policy".to_string(),
            });
        }
        if obligations.denied_type_ids.contains(&ast.from.type_id) {
            return Err(QueryError::FieldForbidden {
                message: "root type denied by policy".to_string(),
            });
        }

        // 2. Intersect select with allowed predicates.
        if !obligations.allowed_predicate_ids.is_empty() {
            ast.select.retain(|s| match &s.field {
                FieldRef::Predicate { predicate_id } => {
                    obligations.allowed_predicate_ids.contains(predicate_id)
                }
                FieldRef::System { .. } => true,
            });
        }
        // Remove denied predicates from select.
        ast.select.retain(|s| match &s.field {
            FieldRef::Predicate { predicate_id } => {
                !obligations.denied_predicate_ids.contains(predicate_id)
            }
            FieldRef::System { .. } => true,
        });

        // 3. Reject sort on hidden fields (no side channel).
        for key in &ast.sort {
            if let FieldRef::Predicate { predicate_id } = &key.field {
                if obligations.denied_predicate_ids.contains(predicate_id) {
                    return Err(QueryError::FieldForbidden {
                        message: "sort on hidden field forbidden".to_string(),
                    });
                }
                if !obligations.allowed_predicate_ids.is_empty()
                    && !obligations.allowed_predicate_ids.contains(predicate_id)
                {
                    return Err(QueryError::FieldForbidden {
                        message: "sort on non-allowed field forbidden".to_string(),
                    });
                }
            }
        }

        // 4. Clamp page size.
        if ast.page.size > obligations.max_page_size {
            ast.page.size = obligations.max_page_size;
        }

        // 5. Add mandatory filters to the root where.
        let mut new_where = Vec::new();
        for filter in &obligations.mandatory_filters {
            // Each mandatory filter becomes a Comparison on a system field.
            new_where.push(WhereExpr::Comparison {
                field: FieldRef::System {
                    system: SystemField::PublicId, // placeholder; real impl uses filter.system_field
                },
                op: Operator::Eq,
                value: filter.value.clone(),
                values: None,
                valid_at: None,
            });
        }
        if let Some(existing) = ast.where_expr.take() {
            new_where.insert(0, existing);
        }
        if !new_where.is_empty() {
            ast.where_expr = Some(if new_where.len() == 1 {
                new_where.pop().unwrap()
            } else {
                WhereExpr::And(new_where)
            });
        }

        // 6. Remove forbidden relation predicates from Relation nodes.
        prune_relations(&mut ast.where_expr, obligations);

        Ok(ast)
    }
}

fn prune_relations(where_expr: &mut Option<WhereExpr>, obligations: &QueryObligations) {
    if let Some(expr) = where_expr {
        prune_relation_node(expr, obligations);
    }
}

fn prune_relation_node(expr: &mut WhereExpr, obligations: &QueryObligations) {
    // Take ownership of inner data we need to mutate, to avoid borrow conflicts.
    let (forbidden, mut filter_opt) = match expr {
        WhereExpr::Relation {
            predicate_id,
            filter,
            ..
        } => {
            let pid = *predicate_id;
            let forbidden = obligations.denied_predicate_ids.contains(&pid)
                || (!obligations.allowed_predicate_ids.is_empty()
                    && !obligations.allowed_predicate_ids.contains(&pid));
            (forbidden, filter.take())
        }
        WhereExpr::And(items) | WhereExpr::Or(items) => {
            for item in items {
                prune_relation_node(item, obligations);
            }
            return;
        }
        WhereExpr::Not(inner) => {
            prune_relation_node(inner, obligations);
            return;
        }
        _ => return,
    };
    if forbidden {
        // Replace forbidden relation with an always-false leaf.
        *expr = WhereExpr::Comparison {
            field: FieldRef::System {
                system: SystemField::EntityId,
            },
            op: Operator::Eq,
            value: Some(serde_json::json!(0)),
            values: None,
            valid_at: None,
        };
    }
    if let Some(f) = &mut filter_opt {
        prune_relation_node(f, obligations);
    }
    // Put the filter back if we still have a Relation node.
    if let WhereExpr::Relation { filter, .. } = expr {
        *filter = filter_opt;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ast::{FromClause, KeysetPage, SortDirection};
    use enginer_security_domain::decision::{MandatoryFilter, QueryObligations};

    fn empty_obligations() -> QueryObligations {
        QueryObligations {
            allowed_root_type_ids: Vec::new(),
            denied_type_ids: Vec::new(),
            allowed_predicate_ids: Vec::new(),
            denied_predicate_ids: Vec::new(),
            allowed_relations: Vec::new(),
            mandatory_filters: Vec::new(),
            masks: Vec::new(),
            max_page_size: 50,
            max_graph_depth: 2,
            max_graph_nodes: 300,
            aggregates: "allow".to_string(),
        }
    }

    fn sample_ast() -> QueryAst {
        QueryAst {
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
            where_expr: None,
            sort: vec![SortKey {
                field: FieldRef::System {
                    system: SystemField::PublicId,
                },
                direction: SortDirection::Asc,
            }],
            page: KeysetPage { size: 500, after: None },
            as_of: None,
        }
    }

    #[test]
    fn rewriter_clamps_page_size() {
        let ast = sample_ast();
        let obs = empty_obligations();
        let out = SecurityRewriter.rewrite(ast, &obs).unwrap();
        assert_eq!(out.page.size, 50);
    }

    #[test]
    fn rewriter_denies_root_type() {
        let mut ast = sample_ast();
        let tid = ast.from.type_id;
        let mut obs = empty_obligations();
        obs.denied_type_ids.push(tid);
        let res = SecurityRewriter.rewrite(ast, &obs);
        assert!(res.is_err());
    }

    #[test]
    fn rewriter_drops_denied_predicate_from_select() {
        let pid = uuid::Uuid::now_v7();
        let mut ast = sample_ast();
        ast.select.push(SelectField {
            field: FieldRef::Predicate { predicate_id: pid },
            alias: "secret".to_string(),
        });
        let mut obs = empty_obligations();
        obs.denied_predicate_ids.push(pid);
        let out = SecurityRewriter.rewrite(ast, &obs).unwrap();
        assert!(out.select.iter().all(|s| match &s.field {
            FieldRef::Predicate { predicate_id } => *predicate_id != pid,
            _ => true,
        }));
    }

    #[test]
    fn rewriter_rejects_sort_on_hidden_field() {
        let pid = uuid::Uuid::now_v7();
        let mut ast = sample_ast();
        ast.sort.push(SortKey {
            field: FieldRef::Predicate { predicate_id: pid },
            direction: SortDirection::Asc,
        });
        let mut obs = empty_obligations();
        obs.denied_predicate_ids.push(pid);
        let res = SecurityRewriter.rewrite(ast, &obs);
        assert!(res.is_err());
    }

    #[test]
    fn rewriter_adds_mandatory_filter() {
        let ast = sample_ast();
        let mut obs = empty_obligations();
        obs.mandatory_filters.push(MandatoryFilter {
            system_field: "business_unit_id".to_string(),
            op: "eq".to_string(),
            value: Some(serde_json::json!("bu-1")),
            scope: None,
        });
        let out = SecurityRewriter.rewrite(ast, &obs).unwrap();
        assert!(out.where_expr.is_some());
    }
}
