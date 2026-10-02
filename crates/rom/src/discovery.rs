use super::*;

/// The metadata item a trusted discovery policy may disclose. A grant does not
/// authorize reading data, using a predicate, or invoking an operation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DiscoveryTarget<'a> {
    Resource,
    Field(&'a str),
    Action(&'a str),
}
pub(crate) type DiscoveryPolicy = Arc<dyn Fn(&Actor, DiscoveryTarget<'_>) -> bool + Send + Sync>;

/// Currently authorized metadata, with no inferred row or mutation permissions.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct Discovery {
    pub version: u32,
    pub resources: Vec<DiscoveredResource>,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct DiscoveredResource {
    pub kind: String,
    pub version: u32,
    pub fields: Vec<DiscoveredField>,
    /// Input codecs remain opaque; these names imply no input schema or grant.
    pub actions: Vec<String>,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct DiscoveredField {
    pub name: String,
    pub shape: Shape,
}

// Borrow accepted descriptors until their complete wire representation fits.
// Building a full cloned catalog before measuring would defeat this bound.
#[derive(Serialize)]
struct ResourceHeader<'a> {
    kind: &'a str,
    version: u32,
    fields: [(); 0],
    actions: [(); 0],
}
#[derive(Serialize)]
struct FieldMetadata<'a> {
    name: &'a str,
    shape: &'a Shape,
}
struct Budget(usize);
impl std::io::Write for Budget {
    fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
        self.0 = self
            .0
            .checked_sub(bytes.len())
            .ok_or_else(|| std::io::Error::other("discovery limit"))?;
        Ok(bytes.len())
    }
    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}
impl Budget {
    fn charge(&mut self, value: &impl Serialize) -> Result<()> {
        serde_json::to_writer(self, value).map_err(|_| Error::TooLarge)
    }
    fn comma(&mut self, preceding: usize) -> Result<()> {
        if preceding != 0 {
            self.0 = self.0.checked_sub(1).ok_or(Error::TooLarge)?;
        }
        Ok(())
    }
}
fn visible_references(shape: &Shape, kinds: &BTreeSet<&str>) -> bool {
    match shape {
        Shape::Reference { kind } => kinds.contains(kind.as_str()),
        Shape::Nullable(inner)
        | Shape::Optional(inner)
        | Shape::List(inner)
        | Shape::Map(inner) => visible_references(inner, kinds),
        _ => true,
    }
}
impl Runtime {
    /// Disclose only explicitly granted metadata under current actor authority.
    /// No rows, codecs or domain policies are consulted. The entire response must
    /// fit `Limits::snapshot_bytes`; oversized catalogs fail without truncation.
    pub async fn discover(&self, actor: &Actor) -> Result<Discovery> {
        let actor_for_policy = actor.clone();
        self.observe(actor, move |runtime| {
            let mut result = Discovery {
                version: 1,
                resources: Vec::new(),
            };
            let mut budget = Budget(runtime.0.limits.snapshot_bytes);
            budget.charge(&result)?;
            let mut visible = Vec::new();
            let mut kinds = BTreeSet::new();
            for (kind, definition) in &runtime.0.registry {
                if !definition.allows_discovery(&actor_for_policy, DiscoveryTarget::Resource) {
                    continue;
                }
                let descriptor = definition.descriptor_ref();
                budget.comma(visible.len())?;
                budget.charge(&ResourceHeader {
                    kind,
                    version: descriptor.version,
                    fields: [],
                    actions: [],
                })?;
                kinds.insert(kind.as_str());
                visible.push(definition);
            }
            for definition in visible {
                let descriptor = definition.descriptor_ref();
                let mut resource = DiscoveredResource {
                    kind: descriptor.kind.clone(),
                    version: descriptor.version,
                    fields: Vec::new(),
                    actions: Vec::new(),
                };
                for field in &descriptor.fields {
                    if !definition
                        .allows_discovery(&actor_for_policy, DiscoveryTarget::Field(&field.name))
                        || !visible_references(&field.shape, &kinds)
                    {
                        continue;
                    }
                    budget.comma(resource.fields.len())?;
                    budget.charge(&FieldMetadata {
                        name: &field.name,
                        shape: &field.shape,
                    })?;
                    resource.fields.push(DiscoveredField {
                        name: field.name.clone(),
                        shape: field.shape.clone(),
                    });
                }
                resource.fields.sort_by(|a, b| a.name.cmp(&b.name));
                for name in definition.actions().keys() {
                    if definition.allows_discovery(&actor_for_policy, DiscoveryTarget::Action(name))
                    {
                        budget.comma(resource.actions.len())?;
                        budget.charge(name)?;
                        resource.actions.push(name.clone());
                    }
                }
                result.resources.push(resource);
            }
            Ok(result)
        })
        .await
    }
}
