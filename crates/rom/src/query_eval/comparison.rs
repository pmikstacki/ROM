//! Canonical predicate matching, order keys and value comparison.
use super::Plan;
use crate::{AnchorValue, CompareOp, Direction, Error, Result, Shape, Value, matches_shape};
use std::cmp::Ordering as Cmp;
fn compare_value(a: Option<&Value>, b: Option<&Value>, shape: &Shape) -> Result<Cmp> {
    match (a, b) {
        (None, None) => return Ok(Cmp::Equal),
        (None, Some(_)) => return Ok(Cmp::Less),
        (Some(_), None) => return Ok(Cmp::Greater),
        (Some(Value::Null), Some(Value::Null)) => return Ok(Cmp::Equal),
        (Some(Value::Null), _) => return Ok(Cmp::Less),
        (_, Some(Value::Null)) => return Ok(Cmp::Greater),
        _ => {}
    }
    let (a, b) = (a.ok_or(Error::Storage)?, b.ok_or(Error::Storage)?);
    let invalid = || Error::Storage;
    Ok(match shape {
        Shape::Optional(s) | Shape::Nullable(s) => return compare_value(Some(a), Some(b), s),
        Shape::String | Shape::Enum(_) | Shape::Reference { .. } => a
            .as_str()
            .ok_or_else(invalid)?
            .as_bytes()
            .cmp(b.as_str().ok_or_else(invalid)?.as_bytes()),
        Shape::Bool => a
            .as_bool()
            .ok_or_else(invalid)?
            .cmp(&b.as_bool().ok_or_else(invalid)?),
        Shape::U64 => a
            .as_u64()
            .ok_or_else(invalid)?
            .cmp(&b.as_u64().ok_or_else(invalid)?),
        Shape::I64 => a
            .as_i64()
            .ok_or_else(invalid)?
            .cmp(&b.as_i64().ok_or_else(invalid)?),
        Shape::F64 => a
            .as_f64()
            .ok_or_else(invalid)?
            .partial_cmp(&b.as_f64().ok_or_else(invalid)?)
            .ok_or_else(invalid)?,
        _ => return Err(Error::Storage),
    })
}
fn equal(a: &Value, b: &Value, shape: &Shape) -> Result<bool> {
    if shape.is_scalar() {
        Ok(compare_value(Some(a), Some(b), shape)? == Cmp::Equal)
    } else {
        Ok(a == b)
    }
}
impl Plan {
    pub(super) fn matches(&self, value: &Value) -> Result<bool> {
        for f in &self.spec.filters {
            let actual = value.get(&f.field);
            if f.absent {
                if actual.is_some() {
                    return Ok(false);
                }
            } else if !actual
                .map(|v| equal(v, &f.value, &self.shapes[&f.field]))
                .transpose()?
                .unwrap_or(false)
            {
                return Ok(false);
            }
        }
        for p in &self.spec.comparisons {
            let actual = value.get(&p.field);
            if p.absent {
                if (actual.is_none()) != (p.op == CompareOp::Eq) {
                    return Ok(false);
                }
                continue;
            }
            let Some(actual) = actual else {
                return Ok(false);
            };
            let shape = &self.shapes[&p.field];
            let matched = match p.op {
                CompareOp::Eq => equal(actual, &p.value, shape)?,
                CompareOp::Ne => !equal(actual, &p.value, shape)?,
                _ if actual.is_null() => false,
                op => {
                    let c = compare_value(Some(actual), Some(&p.value), shape)?;
                    match op {
                        CompareOp::Lt => c == Cmp::Less,
                        CompareOp::Le => c != Cmp::Greater,
                        CompareOp::Gt => c == Cmp::Greater,
                        CompareOp::Ge => c != Cmp::Less,
                        _ => unreachable!(),
                    }
                }
            };
            if !matched {
                return Ok(false);
            }
        }
        Ok(true)
    }
    pub(super) fn keys(&self, value: &Value) -> Result<Vec<AnchorValue>> {
        self.spec
            .order
            .iter()
            .map(|o| {
                let v = value.get(&o.field);
                let shape = &self.shapes[&o.field];
                if !v.map_or(matches!(shape, Shape::Optional(_)), |v| {
                    matches_shape(v, shape)
                }) {
                    return Err(Error::Storage);
                }
                Ok(v.map_or(AnchorValue::Missing, |v| AnchorValue::Value(v.clone())))
            })
            .collect()
    }
    pub(super) fn compare_keys(
        &self,
        a: &[AnchorValue],
        aid: &str,
        b: &[AnchorValue],
        bid: &str,
    ) -> Result<Cmp> {
        for ((a, b), o) in a.iter().zip(b).zip(&self.spec.order) {
            fn value(a: &AnchorValue) -> Option<&Value> {
                match a {
                    AnchorValue::Missing => None,
                    AnchorValue::Value(v) => Some(v),
                }
            }
            let c = compare_value(value(a), value(b), &self.shapes[&o.field])?;
            let c = if o.direction == Direction::Desc {
                c.reverse()
            } else {
                c
            };
            if c != Cmp::Equal {
                return Ok(c);
            }
        }
        Ok(aid.as_bytes().cmp(bid.as_bytes()))
    }
}
