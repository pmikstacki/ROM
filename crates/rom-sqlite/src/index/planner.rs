//! Conservative SQLite plan recognition and uncalibrated relative cost heuristics.
use super::encoding::encode;
use rom::{CompareOp, QueryCost, QueryEstimates, ReadBinding, StorageQuery};
use rusqlite::{Connection, params_from_iter, types::Value};

pub(super) struct NativePlan {
    pub sql: String,
    pub parameters: Vec<Value>,
    equality: bool,
}

impl NativePlan {
    /// All predicates are conjunctions. One leading predicate yields a complete
    /// candidate superset; ROM evaluates every predicate and pagination afterward.
    pub(super) fn for_request(request: &StorageQuery) -> Option<Self> {
        for filter in &request.spec.filters {
            if let Some(plan) = Self::predicate(
                request,
                &filter.field,
                CompareOp::Eq,
                &filter.value,
                filter.absent,
            ) {
                return Some(plan);
            }
        }
        for comparison in &request.spec.comparisons {
            if let Some(plan) = Self::predicate(
                request,
                &comparison.field,
                comparison.op,
                &comparison.value,
                comparison.absent,
            ) {
                return Some(plan);
            }
        }
        None
    }

    fn predicate(
        request: &StorageQuery,
        field: &str,
        op: CompareOp,
        value: &rom::Value,
        absent: bool,
    ) -> Option<Self> {
        let shape = &request
            .descriptor
            .fields
            .iter()
            .find(|f| f.name == field)?
            .shape;
        if !shape.is_scalar() {
            return None;
        }
        let (operator, key, present_only) = if absent {
            match op {
                CompareOp::Eq => ("=", encode(shape, None).ok()?, false),
                CompareOp::Ne => (">=", vec![1], false),
                _ => return None,
            }
        } else {
            let operator = match op {
                CompareOp::Eq => "=",
                CompareOp::Lt => "<",
                CompareOp::Le => "<=",
                CompareOp::Gt => ">",
                CompareOp::Ge => ">=",
                CompareOp::Ne => return None,
            };
            (
                operator,
                encode(shape, Some(value)).ok()?,
                op != CompareOp::Eq,
            )
        };
        let mut sql = format!(
            "SELECT r.kind,r.id,r.data FROM query_keys k JOIN resources r ON r.kind=k.kind AND r.id=k.id WHERE k.kind=? AND k.field=? AND k.encoded{operator}?"
        );
        let mut parameters = vec![
            Value::Text(request.descriptor.kind.clone()),
            Value::Text(field.to_owned()),
            Value::Blob(key),
        ];
        if present_only {
            // Range predicates never match a missing or null field in ROM.
            sql.push_str(" AND k.encoded>=?");
            parameters.push(Value::Blob(vec![2]));
        }
        Some(Self {
            sql,
            parameters,
            equality: operator == "=",
        })
    }

    pub(super) fn estimates(
        &self,
        c: &Connection,
        binding: ReadBinding,
        rows: u64,
        bytes: u64,
    ) -> Option<QueryEstimates> {
        if !self.recognized(c)? {
            return None;
        }
        // These weights and selectivity fractions are conservative *heuristics*,
        // not calibrated latency measurements or statistics from SQLite. They
        // choose work placement only; admission and result correctness are exact.
        let divisor = if self.equality { 8 } else { 2 };
        let candidates = rows.checked_add(divisor - 1)?.checked_div(divisor)?;
        let candidate_bytes = bytes.checked_add(divisor - 1)?.checked_div(divisor)?;
        Some(QueryEstimates {
            binding,
            complete_candidates: true,
            reference: QueryCost {
                startup: 16,
                rows,
                per_row: 100,
                bytes,
                per_byte: 1,
            },
            native: QueryCost {
                startup: 128,
                rows: candidates,
                per_row: 160,
                bytes: candidate_bytes,
                per_byte: 1,
            },
        })
    }

    fn recognized(&self, c: &Connection) -> Option<bool> {
        // EXPLAIN QUERY PLAN is not a stable wire API. Unknown engines or nodes
        // disable this optional estimate, rather than guessing their meaning.
        if rusqlite::version() != "3.53.2" {
            return None;
        }
        let mut statement = c
            .prepare(&format!("EXPLAIN QUERY PLAN {}", self.sql))
            .ok()?;
        let mut cursor = statement.query(params_from_iter(&self.parameters)).ok()?;
        let mut index = false;
        let mut lookup = false;
        while let Some(row) = cursor.next().ok()? {
            let detail: String = row.get(3).ok()?;
            if matches!(
                detail.as_str(),
                "SEARCH k USING COVERING INDEX query_keys_value (kind=? AND field=? AND encoded=?)"
                    | "SEARCH k USING COVERING INDEX query_keys_value (kind=? AND field=? AND encoded>?)"
                    | "SEARCH k USING COVERING INDEX query_keys_value (kind=? AND field=? AND encoded<?)"
                    | "SEARCH k USING COVERING INDEX query_keys_value (kind=? AND field=? AND encoded>? AND encoded<?)"
            ) {
                if index {
                    return None;
                }
                index = true;
            } else if detail
                == "SEARCH r USING INDEX sqlite_autoindex_resources_1 (kind=? AND id=?)"
            {
                if lookup {
                    return None;
                }
                lookup = true;
            } else {
                return None;
            }
        }
        Some(index && lookup)
    }
}
