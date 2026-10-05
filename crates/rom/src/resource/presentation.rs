//! Advisory authoring metadata, separate from value codecs and persisted schema identity.
use crate::{Descriptor, Error, FieldCodec, Result, Shape};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

/// A human field label, explanation and optional declared group.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FieldPresentation {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub help: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub group: Option<String>,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PresentationGroup {
    pub name: String,
    pub label: String,
}
/// A Settings navigation classification. This grants no operation or configuration authority.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SettingsPresentation {
    pub group: String,
    pub label: String,
}
/// Presentation uses authorized projected values; it cannot request hidden fields.
/// Text is bounded in UTF-8 bytes: labels and selectors at 256, help at 2048.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ResourcePresentation {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub title_field: Option<String>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub fields: BTreeMap<String, FieldPresentation>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub groups: Vec<PresentationGroup>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub settings: Option<SettingsPresentation>,
}
impl ResourcePresentation {
    pub(crate) fn validate(
        &self,
        descriptor: &Descriptor,
        codecs: &BTreeMap<String, FieldCodec>,
    ) -> Result<()> {
        let invalid = || Error::invalid(&descriptor.kind, "presentation");
        let text = |value: &str, limit: usize| {
            if value.is_empty() || value.len() > limit {
                Err(invalid())
            } else {
                Ok(())
            }
        };
        if self.fields.len() > 1024 || self.groups.len() > 1024 {
            return Err(invalid());
        }
        if let Some(label) = &self.label {
            text(label, 256)?;
        }
        if let Some(settings) = &self.settings {
            text(&settings.group, 256)?;
            text(&settings.label, 256)?;
        }
        let mut groups = BTreeSet::new();
        for group in &self.groups {
            text(&group.name, 256)?;
            text(&group.label, 256)?;
            if !groups.insert(&group.name) {
                return Err(invalid());
            }
        }
        for (name, field) in &self.fields {
            if !descriptor.fields.iter().any(|f| &f.name == name) {
                return Err(invalid());
            }
            if let Some(label) = &field.label {
                text(label, 256)?;
            }
            if let Some(help) = &field.help {
                text(help, 2048)?;
            }
            if let Some(group) = &field.group
                && !groups.contains(group)
            {
                return Err(invalid());
            }
        }
        if let Some(name) = &self.title_field {
            text(name, 256)?;
            let field = descriptor
                .fields
                .iter()
                .find(|f| &f.name == name)
                .ok_or_else(invalid)?;
            let mut shape = &field.shape;
            while let Shape::Optional(inner) | Shape::Nullable(inner) = shape {
                shape = inner;
            }
            if !matches!(shape, Shape::String) || codecs.contains_key(name) {
                return Err(invalid());
            }
        }
        Ok(())
    }
    pub(crate) fn project<'a>(&'a self, visible: &BTreeSet<&str>) -> ProjectedPresentation<'a> {
        let fields: BTreeMap<_, _> = self
            .fields
            .iter()
            .filter(|(name, _)| visible.contains(name.as_str()))
            .map(|(name, hint)| (name.as_str(), hint))
            .collect();
        let groups = self
            .groups
            .iter()
            .filter(|group| {
                fields
                    .values()
                    .any(|field| field.group.as_ref() == Some(&group.name))
            })
            .collect();
        ProjectedPresentation {
            label: self.label.as_deref(),
            title_field: self
                .title_field
                .as_deref()
                .filter(|name| visible.contains(name)),
            fields,
            groups,
            settings: self.settings.as_ref(),
        }
    }
}
// Borrow strings until discovery has charged the complete authorized wire metadata.
#[derive(Serialize)]
pub(crate) struct ProjectedPresentation<'a> {
    #[serde(skip_serializing_if = "Option::is_none")]
    label: Option<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    title_field: Option<&'a str>,
    #[serde(skip_serializing_if = "BTreeMap::is_empty")]
    fields: BTreeMap<&'a str, &'a FieldPresentation>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    groups: Vec<&'a PresentationGroup>,
    #[serde(skip_serializing_if = "Option::is_none")]
    settings: Option<&'a SettingsPresentation>,
}
impl ProjectedPresentation<'_> {
    pub(crate) fn into_owned(self) -> ResourcePresentation {
        ResourcePresentation {
            label: self.label.map(str::to_owned),
            title_field: self.title_field.map(str::to_owned),
            fields: self
                .fields
                .into_iter()
                .map(|(name, hint)| (name.to_owned(), hint.clone()))
                .collect(),
            groups: self.groups.into_iter().cloned().collect(),
            settings: self.settings.cloned(),
        }
    }
}
