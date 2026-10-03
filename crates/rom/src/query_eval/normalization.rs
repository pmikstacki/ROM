//! Query admission, canonical operands and anchor construction.
use crate::{
    Actor, AnchorValue, CompareOp, Descriptor, Error, QueryAnchor, QuerySpec, Result, Runtime,
    Shape, Value, matches_shape,
};
use serde::Serialize;
use std::collections::{BTreeMap, BTreeSet};
pub(crate) struct Plan {
    pub(crate) spec: QuerySpec,
    pub(super) shapes: BTreeMap<String, Shape>,
}
fn field<'a>(d: &'a Descriptor, name: &str) -> Result<&'a Shape> {
    d.fields
        .iter()
        .find(|f| f.name == name)
        .map(|f| &f.shape)
        .ok_or_else(|| Error::invalid(&d.kind, name))
}
fn normalized(
    d: &Descriptor,
    name: &str,
    value: Value,
    codec: &impl Fn(&str, Value) -> Result<Value>,
) -> Result<Value> {
    let shape = field(d, name)?;
    if !matches_shape(&value, shape) {
        return Err(Error::invalid(&d.kind, name));
    }
    let value = codec(name, value)?;
    if !matches_shape(&value, shape) {
        return Err(Error::invalid(&d.kind, name));
    }
    Ok(value)
}
pub(crate) fn normalize(
    d: &Descriptor,
    query: &QuerySpec,
    codec: impl Fn(&str, Value) -> Result<Value>,
) -> Result<Plan> {
    if query.filters.len().saturating_add(query.comparisons.len()) > 32 || query.order.len() > 4 {
        return Err(Error::TooLarge);
    }
    if query.after_id.is_some() && (!query.order.is_empty() || query.after.is_some()) {
        return Err(Error::invalid(&d.kind, "after_id"));
    }
    let mut spec = query.clone();
    let mut shapes = BTreeMap::new();
    for f in &mut spec.filters {
        let shape = field(d, &f.field)?;
        shapes.insert(f.field.clone(), shape.clone());
        if f.absent {
            if !f.value.is_null() || !matches!(shape, Shape::Optional(_)) {
                return Err(Error::invalid(&d.kind, &f.field));
            }
        } else {
            f.value = normalized(d, &f.field, f.value.clone(), &codec)?;
        }
    }
    for p in &mut spec.comparisons {
        let shape = field(d, &p.field)?;
        shapes.insert(p.field.clone(), shape.clone());
        if p.absent {
            if !p.value.is_null()
                || !matches!(shape, Shape::Optional(_))
                || !matches!(p.op, CompareOp::Eq | CompareOp::Ne)
            {
                return Err(Error::invalid(&d.kind, &p.field));
            }
        } else {
            p.value = normalized(d, &p.field, p.value.clone(), &codec)?;
            if !matches!(p.op, CompareOp::Eq | CompareOp::Ne)
                && (!shape.is_scalar() || p.value.is_null())
            {
                return Err(Error::invalid(&d.kind, &p.field));
            }
        }
    }
    let mut sorted = BTreeSet::new();
    for o in &spec.order {
        let shape = field(d, &o.field)?;
        if !shape.is_scalar() || !sorted.insert(&o.field) {
            return Err(Error::invalid(&d.kind, &o.field));
        }
        shapes.insert(o.field.clone(), shape.clone());
    }
    if let Some(a) = &spec.after {
        if a.version != 1
            || a.kind != d.kind
            || a.schema_version != d.version
            || a.filters != spec.filters
            || a.comparisons != spec.comparisons
            || a.order != spec.order
            || a.values.len() != spec.order.len()
            || a.id.is_empty()
        {
            return Err(Error::invalid(&d.kind, "anchor"));
        }
        for (key, order) in a.values.iter().zip(&spec.order) {
            let shape = field(d, &order.field)?;
            match key {
                AnchorValue::Missing if matches!(shape, Shape::Optional(_)) => {}
                AnchorValue::Value(v) if normalized(d, &order.field, v.clone(), &codec)? == *v => {}
                _ => return Err(Error::invalid(&d.kind, "anchor")),
            }
        }
    }
    Ok(Plan { spec, shapes })
}
pub(crate) fn make_anchor(
    d: &Descriptor,
    plan: &Plan,
    id: &str,
    value: &Value,
    codec: impl Fn(&str, Value) -> Result<Value>,
) -> Result<QueryAnchor> {
    if id.is_empty() {
        return Err(Error::invalid(&d.kind, "anchor"));
    }
    let mut values = Vec::new();
    for o in &plan.spec.order {
        let shape = field(d, &o.field)?;
        values.push(match value.get(&o.field) {
            Some(v) => AnchorValue::Value(normalized(d, &o.field, v.clone(), &codec)?),
            None if matches!(shape, Shape::Optional(_)) => AnchorValue::Missing,
            None => return Err(Error::invalid(&d.kind, &o.field)),
        });
    }
    Ok(QueryAnchor {
        version: 1,
        kind: d.kind.clone(),
        schema_version: d.version,
        filters: plan.spec.filters.clone(),
        comparisons: plan.spec.comparisons.clone(),
        order: plan.spec.order.clone(),
        id: id.into(),
        values,
    })
}
impl Runtime {
    pub(super) fn query_plan(&self, actor: &Actor, kind: &str, query: &QuerySpec) -> Result<Plan> {
        if query
            .limit
            .is_some_and(|n| n == 0 || n > self.0.limits.snapshot_rows)
        {
            return Err(Error::TooLarge);
        }
        check_size(&(kind, query), self.0.limits.command_bytes)?;
        let def = self.0.registry.get(kind).ok_or(Error::Unregistered)?;
        for name in query
            .filters
            .iter()
            .map(|p| p.field.as_str())
            .chain(query.comparisons.iter().map(|p| p.field.as_str()))
        {
            if !def.allows_query(actor, name) {
                return Err(Error::Denied);
            }
        }
        for o in &query.order {
            if !def.allows_sort(actor, &o.field) {
                return Err(Error::Denied);
            }
        }
        let plan = normalize(def.descriptor_ref(), query, |name, value| {
            def.normalize_field(name, value)
        })?;
        check_size(&plan.spec, self.0.limits.command_bytes)?;
        Ok(plan)
    }
}
pub(super) fn check_size(value: &impl Serialize, limit: usize) -> Result<()> {
    struct Counter(usize);
    impl std::io::Write for Counter {
        fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
            self.0 = self
                .0
                .checked_sub(bytes.len())
                .ok_or_else(|| std::io::Error::other("query limit"))?;
            Ok(bytes.len())
        }
        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }
    serde_json::to_writer(Counter(limit), value).map_err(|_| Error::TooLarge)
}
