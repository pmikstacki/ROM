//! Descriptor-based integrity shared by all persistence adapters.
use super::*;

/// One deduplicated live reference. Historical values do not produce these edges.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReferenceEdge {
    pub source: Key,
    pub target: Key,
}

impl Descriptor {
    /// Validate a layout and sort fields by name for stable persisted comparison.
    /// This does not certify custom codec behavior or validate target registration.
    pub fn canonical(&self) -> Result<Self> {
        if self.kind.is_empty() || self.version == 0 {
            return Err(Error::invalid(&self.kind, "descriptor"));
        }
        let mut descriptor = self.clone();
        descriptor.fields.sort_by(|a, b| a.name.cmp(&b.name));
        let mut previous = None;
        for field in &descriptor.fields {
            if field.name.is_empty() || previous == Some(&field.name) {
                return Err(Error::invalid(&self.kind, "descriptor"));
            }
            validate_shape(&field.shape, 0, None)?;
            previous = Some(&field.name);
        }
        Ok(descriptor)
    }

    /// Validate a canonical Resource value and return its sorted, unique targets.
    /// None denotes a tombstone. It contributes no live references.
    pub fn reference_targets(&self, value: Option<&Value>) -> Result<Vec<Key>> {
        let descriptor = self.canonical()?;
        let Some(value) = value else {
            return Ok(Vec::new());
        };
        let map = resource::canonical_fields(&descriptor, value)?;
        let mut targets = BTreeSet::new();
        for field in &descriptor.fields {
            let Some(value) = map.get(&field.name) else {
                continue;
            };
            collect_targets(&field.shape, value, &mut targets);
        }
        Ok(targets.into_iter().collect())
    }
}

/// Validate a complete persisted catalog, including target kinds.
/// Adapters merge new definitions with retained definitions before this call.
pub fn validate_descriptors(descriptors: &[Descriptor]) -> Result<Vec<Descriptor>> {
    let mut catalog = BTreeMap::new();
    for descriptor in descriptors {
        let descriptor = descriptor.canonical()?;
        if catalog
            .insert(descriptor.kind.clone(), descriptor)
            .is_some()
        {
            return Err(Error::Duplicate("persisted Resource kind".into()));
        }
    }
    let kinds = catalog.keys().cloned().collect();
    for descriptor in catalog.values() {
        for field in &descriptor.fields {
            validate_shape(&field.shape, 0, Some(&kinds))?;
        }
    }
    Ok(catalog.into_values().collect())
}

fn collect_targets(shape: &Shape, value: &Value, targets: &mut BTreeSet<Key>) {
    match shape {
        Shape::Reference { kind } => {
            // The complete value was checked against its shape before traversal.
            if let Some(id) = value.as_str() {
                targets.insert(Key {
                    kind: kind.clone(),
                    id: id.into(),
                });
            }
        }
        Shape::Optional(inner) | Shape::Nullable(inner) if !value.is_null() => {
            collect_targets(inner, value, targets);
        }
        Shape::List(inner) => {
            if let Some(values) = value.as_array() {
                for value in values {
                    collect_targets(inner, value, targets);
                }
            }
        }
        Shape::Map(inner) => {
            if let Some(values) = value.as_object() {
                for value in values.values() {
                    collect_targets(inner, value, targets);
                }
            }
        }
        _ => {}
    }
}
