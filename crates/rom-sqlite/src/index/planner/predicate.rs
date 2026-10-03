//! One encoded predicate supplies both probe and complete materialization SQL.
use super::{MAX_PREDICATES, NativePlan};
use crate::index::encoding::encode;
use rom::{CompareOp, StorageQuery};
use rusqlite::types::Value;

#[derive(PartialEq)]
pub(super) struct Predicate {
    condition: String,
    pub parameters: Vec<Value>,
}

impl Predicate {
    pub(super) fn for_request(request: &StorageQuery) -> Vec<Self> {
        let mut predicates = Vec::new();
        let mut add = |field: &str, op, value: &rom::Value, absent| {
            if let Some(predicate) = Self::new(request, field, op, value, absent)
                && !predicates.contains(&predicate)
            {
                predicates.push(predicate);
            }
            predicates.len() == MAX_PREDICATES
        };
        for filter in &request.spec.filters {
            if add(&filter.field, CompareOp::Eq, &filter.value, filter.absent) {
                return predicates;
            }
        }
        for comparison in &request.spec.comparisons {
            if add(
                &comparison.field,
                comparison.op,
                &comparison.value,
                comparison.absent,
            ) {
                break;
            }
        }
        predicates
    }

    fn new(
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
        let mut condition = format!("k.kind=? AND k.field=? AND k.encoded{operator}?");
        let mut parameters = vec![
            Value::Text(request.descriptor.kind.clone()),
            Value::Text(field.to_owned()),
            Value::Blob(key),
        ];
        if present_only {
            condition.push_str(" AND k.encoded>=?");
            parameters.push(Value::Blob(vec![2]));
        }
        Some(Self {
            condition,
            parameters,
        })
    }

    pub(super) fn probe_sql(&self) -> String {
        format!(
            "SELECT 1 FROM query_keys k WHERE {} LIMIT ?",
            self.condition
        )
    }

    pub(super) fn materialization(self) -> NativePlan {
        NativePlan {
            sql: format!(
                "SELECT r.kind,r.id,r.data FROM query_keys k JOIN resources r ON r.kind=k.kind AND r.id=k.id WHERE {}",
                self.condition
            ),
            parameters: self.parameters,
        }
    }
}
