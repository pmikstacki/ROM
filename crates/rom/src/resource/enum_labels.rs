//! Advisory enum labels, accepted and frozen separately from persisted schema identity.
use crate::{Descriptor, Error, Result, Shape};
use std::collections::BTreeMap;

/// One Resource field's labels for exact wire values at its enum leaf.
/// Labels do not change allowed values, ordering, codecs, or authority.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FieldEnumLabels {
    pub name: String,
    pub labels: BTreeMap<String, String>,
}

pub(crate) fn validate_labels(shape: &Shape, labels: &BTreeMap<String, String>) -> Result<()> {
    if labels.is_empty() {
        return Ok(());
    }
    let mut leaf = shape;
    while let Shape::Optional(inner)
    | Shape::Nullable(inner)
    | Shape::List(inner)
    | Shape::Map(inner) = leaf
    {
        leaf = inner;
    }
    let Shape::Enum(values) = leaf else {
        return Err(Error::invalid("input", "enum labels"));
    };
    if labels.len() > 1024
        || labels
            .iter()
            .any(|(value, label)| !values.contains(value) || label.is_empty() || label.len() > 256)
    {
        return Err(Error::invalid("input", "enum labels"));
    }
    Ok(())
}

pub(crate) fn field_bindings(
    descriptor: &Descriptor,
    bindings: Vec<FieldEnumLabels>,
) -> Result<BTreeMap<String, BTreeMap<String, String>>> {
    let mut result = BTreeMap::new();
    for binding in bindings {
        let field = descriptor
            .fields
            .iter()
            .find(|field| field.name == binding.name)
            .ok_or_else(|| Error::invalid(&descriptor.kind, "enum labels"))?;
        validate_labels(&field.shape, &binding.labels)
            .map_err(|_| Error::invalid(&descriptor.kind, &field.name))?;
        if result.insert(binding.name, binding.labels).is_some() {
            return Err(Error::invalid(&descriptor.kind, "enum labels"));
        }
    }
    Ok(result)
}
