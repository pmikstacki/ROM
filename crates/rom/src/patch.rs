use super::*;
/// A Resource field that may be absent. `Presence<Option<T>>` also permits explicit null.
#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub enum Presence<T> {
    #[default]
    Missing,
    Value(T),
}
impl<T: Field> Field for Presence<T> {
    fn enum_labels() -> BTreeMap<String, String> {
        T::enum_labels()
    }
    fn codec_identity() -> Option<CodecIdentity> {
        T::codec_identity()
    }
    fn codec_wrappers() -> Vec<CodecWrapper> {
        resource::wrapper_path::<T>(CodecWrapper::Optional)
    }
    // Standalone input uses a conditional tagged envelope, not field-value syntax.
    fn input_descriptor() -> Option<InputDescriptor> {
        None
    }
    fn shape() -> Shape {
        Shape::Optional(Box::new(T::shape()))
    }
    // Only encode after is_present; absence is a property of the containing object.
    fn encode(&self) -> Value {
        match self {
            Self::Missing => Value::Null,
            Self::Value(v) => v.encode(),
        }
    }
    fn decode(value: Value) -> Result<Self> {
        T::decode(value).map(Self::Value)
    }
    fn is_present(&self) -> bool {
        matches!(self, Self::Value(_))
    }
    fn decode_missing() -> Result<Self> {
        Ok(Self::Missing)
    }
    fn encode_input(&self) -> Value {
        match self {
            Self::Missing => json!({"presence":"missing"}),
            Self::Value(v) => json!({"presence":"value","value":v.encode()}),
        }
    }
    fn decode_input(value: Value) -> Result<Self> {
        let map = value
            .as_object()
            .ok_or_else(|| Error::invalid("input", "presence"))?;
        match map.get("presence").and_then(Value::as_str) {
            Some("missing") if map.len() == 1 => Ok(Self::Missing),
            Some("value") if map.len() == 2 => <Self as Field>::decode(
                map.get("value")
                    .ok_or_else(|| Error::invalid("input", "value"))?
                    .clone(),
            ),
            _ => Err(Error::invalid("input", "presence")),
        }
    }
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(
    tag = "op",
    content = "value",
    rename_all = "snake_case",
    deny_unknown_fields
)]
pub enum FieldUpdate {
    Set(Value),
    Remove,
}
#[derive(Clone, Debug)]
pub struct Patch<R> {
    pub(crate) fields: BTreeMap<String, FieldUpdate>,
    marker: PhantomData<fn() -> R>,
}
impl<R: Resource> Default for Patch<R> {
    fn default() -> Self {
        Self::new()
    }
}
impl<R: Resource> Patch<R> {
    pub fn new() -> Self {
        Self {
            fields: BTreeMap::new(),
            marker: PhantomData,
        }
    }
    pub fn set<T: Field>(mut self, field: FieldRef<R, T>, value: T) -> Self {
        self.fields.insert(
            field.name.into(),
            if value.is_present() {
                FieldUpdate::Set(value.encode())
            } else {
                FieldUpdate::Remove
            },
        );
        self
    }
    pub fn remove<T: Field>(mut self, field: FieldRef<R, Presence<T>>) -> Self {
        self.fields.insert(field.name.into(), FieldUpdate::Remove);
        self
    }
}
pub(crate) fn normalize_patch(
    def: &dyn Registered,
    fields: &BTreeMap<String, FieldUpdate>,
) -> Result<BTreeMap<String, FieldUpdate>> {
    let descriptor = def.descriptor();
    fields
        .iter()
        .map(|(name, update)| {
            let field = descriptor
                .fields
                .iter()
                .find(|f| &f.name == name)
                .ok_or_else(|| Error::invalid(&descriptor.kind, name))?;
            let update = match update {
                FieldUpdate::Remove if matches!(field.shape, Shape::Optional(_)) => {
                    FieldUpdate::Remove
                }
                FieldUpdate::Remove => return Err(Error::invalid(&descriptor.kind, name)),
                FieldUpdate::Set(value) => {
                    let normalized = def.normalize_field(name, value.clone())?;
                    if !matches_shape(&normalized, &field.shape) {
                        return Err(Error::invalid(&descriptor.kind, name));
                    }
                    FieldUpdate::Set(normalized)
                }
            };
            Ok((name.clone(), update))
        })
        .collect()
}
