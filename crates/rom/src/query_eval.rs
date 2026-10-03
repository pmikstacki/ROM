//! Private canonical plan shared by typed, projected and live observations.
use super::*;
use crate::query_storage::{scalar_shape, validate_query_read};
use std::cmp::Ordering as Cmp;
pub(crate) struct Plan {
    pub(crate) spec: QuerySpec,
    shapes: BTreeMap<String, Shape>,
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
                && (!scalar_shape(shape) || p.value.is_null())
            {
                return Err(Error::invalid(&d.kind, &p.field));
            }
        }
    }
    let mut sorted = BTreeSet::new();
    for o in &spec.order {
        let shape = field(d, &o.field)?;
        if !scalar_shape(shape) || !sorted.insert(&o.field) {
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
    if scalar_shape(shape) {
        Ok(compare_value(Some(a), Some(b), shape)? == Cmp::Equal)
    } else {
        Ok(a == b)
    }
}
impl Plan {
    fn matches(&self, value: &Value) -> Result<bool> {
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
    fn keys(&self, value: &Value) -> Result<Vec<AnchorValue>> {
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
    fn compare_keys(
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
impl Runtime {
    fn query_plan(&self, actor: &Actor, kind: &str, query: &QuerySpec) -> Result<Plan> {
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
    pub(crate) fn select_rows(
        &self,
        actor: &Actor,
        kind: &str,
        query: &QuerySpec,
    ) -> Result<Vec<Row>> {
        let plan = self.query_plan(actor, kind, query)?;
        let def = self.0.registry.get(kind).ok_or(Error::Unregistered)?;
        // Explicit actor-only authorization is one decision per selection attempt.
        // Final disclosure still performs its normal current-authority checks.
        let uniform_read = def.uniform_read(actor);
        if uniform_read == Some(false) {
            return Err(Error::Denied);
        }
        let uniform_fields = uniform_read.is_some() && def.uniform_fields();
        let request = StorageQuery {
            spec: plan.spec.clone(),
            descriptor: def.descriptor(),
            semantics: QUERY_SEMANTICS_VERSION,
            selection: if uniform_fields {
                SelectionMode::UniformReadAndFields
            } else {
                SelectionMode::ReferenceOnly
            },
        };
        let bounds = QueryBounds {
            max_rows: self.0.limits.snapshot_rows,
            max_bytes: self.0.limits.snapshot_bytes,
        };
        let rows = validate_query_read(
            &request,
            bounds,
            self.0.storage.query_read(&request, bounds)?,
        )?;
        let mut selected = Vec::new();
        for row in rows {
            if plan.spec.order.is_empty()
                && (plan
                    .spec
                    .after_id
                    .as_ref()
                    .is_some_and(|id| row.key.id <= *id)
                    || plan.spec.after.as_ref().is_some_and(|a| row.key.id <= a.id))
            {
                continue;
            }
            let Some(value) = row.value.as_ref() else {
                continue;
            };
            if uniform_read.is_none() && !def.allows(actor, Access::Read, value) {
                continue;
            }
            if !uniform_fields {
                for o in &plan.spec.order {
                    if !def.allows_field(actor, Access::Read, &o.field, value) {
                        return Err(Error::Denied);
                    }
                }
            }
            if !plan.matches(value)?
                || plan
                    .spec
                    .after_id
                    .as_ref()
                    .is_some_and(|id| row.key.id <= *id)
            {
                continue;
            }
            let keys = plan.keys(value)?;
            if let Some(anchor) = &plan.spec.after
                && plan.compare_keys(&keys, &row.key.id, &anchor.values, &anchor.id)?
                    != Cmp::Greater
            {
                continue;
            }
            selected.push((row, keys));
            // ID-sorted input already has final order. Keep the legacy page fast path;
            // full snapshot size and duplicate validation still happened above.
            if plan.spec.order.is_empty()
                && selected.len() == plan.spec.limit.unwrap_or(self.0.limits.snapshot_rows)
            {
                break;
            }
        }
        // Every key was validated against its scalar shape above; sorting is infallible.
        if !plan.spec.order.is_empty() {
            selected.sort_by(|(a, ak), (b, bk)| {
                plan.compare_keys(ak, &a.key.id, bk, &b.key.id)
                    .expect("validated query keys")
            });
        }
        selected.truncate(plan.spec.limit.unwrap_or(self.0.limits.snapshot_rows));
        Ok(selected.into_iter().map(|(row, _)| row).collect())
    }
    /// Build a client boundary from supplied projected values under current query/sort grants.
    /// This does not certify origin or current row/field visibility; selection checks those.
    pub async fn query_anchor(
        &self,
        actor: &Actor,
        spec: &QuerySpec,
        view: &ProjectedView,
    ) -> Result<QueryAnchor> {
        let a = actor.clone();
        let spec = spec.clone();
        let view = view.clone();
        self.observe(actor, move |runtime| {
            let plan = runtime.query_plan(&a, &view.key.kind, &spec)?;
            let def = runtime
                .0
                .registry
                .get(&view.key.kind)
                .ok_or(Error::Unregistered)?;
            let value = Value::Object(view.value.clone().ok_or(Error::Missing)?);
            let anchor = make_anchor(def.descriptor_ref(), &plan, &view.key.id, &value, |n, v| {
                def.normalize_field(n, v)
            })?;
            check_size(&anchor, runtime.0.limits.command_bytes)?;
            Ok(anchor)
        })
        .await
    }
}
fn check_size(value: &impl Serialize, limit: usize) -> Result<()> {
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
