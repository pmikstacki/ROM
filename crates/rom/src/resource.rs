use super::*;
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(tag = "type", content = "value", rename_all = "snake_case")]
pub enum Shape {
    String,
    Bool,
    U64,
    I64,
    F64,
    Nullable(Box<Shape>),
    Optional(Box<Shape>),
    List(Box<Shape>),
    Map(Box<Shape>),
    Enum(Vec<String>),
    Reference { kind: String },
}
pub trait Field: Clone + Send + Sync + 'static {
    fn shape() -> Shape;
    fn encode(&self) -> Value;
    fn decode(value: Value) -> Result<Self>;
    fn is_present(&self) -> bool {
        true
    }
    fn decode_missing() -> Result<Self> {
        Err(Error::invalid("input", "missing field"))
    }
    fn encode_input(&self) -> Value {
        self.encode()
    }
    fn decode_input(value: Value) -> Result<Self> {
        Self::decode(value)
    }
}
macro_rules! scalar {
    ($ty:ty,$shape:ident,$get:ident,$map:expr) => {
        impl Field for $ty {
            fn shape() -> Shape {
                Shape::$shape
            }
            fn encode(&self) -> Value {
                json!(self)
            }
            fn decode(value: Value) -> Result<Self> {
                value
                    .$get()
                    .map($map)
                    .ok_or_else(|| Error::invalid("input", "$"))
            }
        }
    };
}
scalar!(String, String, as_str, |s: &str| s.to_owned());
scalar!(bool, Bool, as_bool, |v| v);
scalar!(u64, U64, as_u64, |v| v);
scalar!(i64, I64, as_i64, |v| v);
/// A finite JSON floating-point value. Invalid floats cannot become a nullable null.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FiniteF64(f64);
impl FiniteF64 {
    pub fn new(value: f64) -> Result<Self> {
        if value.is_finite() {
            Ok(Self(value))
        } else {
            Err(Error::invalid("input", "finite number"))
        }
    }
    pub fn get(self) -> f64 {
        self.0
    }
}
impl TryFrom<f64> for FiniteF64 {
    type Error = Error;
    fn try_from(value: f64) -> Result<Self> {
        Self::new(value)
    }
}
impl Field for FiniteF64 {
    fn shape() -> Shape {
        Shape::F64
    }
    fn encode(&self) -> Value {
        json!(self.0)
    }
    fn decode(value: Value) -> Result<Self> {
        value
            .as_f64()
            .ok_or_else(|| Error::invalid("input", "finite number"))
            .and_then(Self::new)
    }
}
impl<T: Field> Field for Vec<T> {
    fn shape() -> Shape {
        Shape::List(Box::new(T::shape()))
    }
    fn encode(&self) -> Value {
        Value::Array(self.iter().map(Field::encode).collect())
    }
    fn decode(value: Value) -> Result<Self> {
        match value {
            Value::Array(values) => values.into_iter().map(T::decode).collect(),
            _ => Err(Error::invalid("input", "$")),
        }
    }
}
impl<T: Field> Field for BTreeMap<String, T> {
    fn shape() -> Shape {
        Shape::Map(Box::new(T::shape()))
    }
    fn encode(&self) -> Value {
        Value::Object(self.iter().map(|(k, v)| (k.clone(), v.encode())).collect())
    }
    fn decode(value: Value) -> Result<Self> {
        match value {
            Value::Object(values) => values
                .into_iter()
                .map(|(k, v)| Ok((k, T::decode(v)?)))
                .collect(),
            _ => Err(Error::invalid("input", "$")),
        }
    }
}
/// A typed identity reference, not a promise of foreign-key integrity or cascade.
#[derive(Clone)]
pub struct ResourceRef<R: Resource> {
    id: String,
    marker: PhantomData<fn() -> R>,
}
impl<R: Resource> ResourceRef<R> {
    pub fn new(id: impl Into<String>) -> Result<Self> {
        let id = id.into();
        if id.is_empty() {
            return Err(Error::invalid(R::KIND, "reference identity"));
        }
        Ok(Self {
            id,
            marker: PhantomData,
        })
    }
    pub fn id(&self) -> &str {
        &self.id
    }
}
impl<R: Resource> std::fmt::Debug for ResourceRef<R> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ResourceRef")
            .field("kind", &R::KIND)
            .field("id", &self.id)
            .finish()
    }
}
impl<R: Resource> PartialEq for ResourceRef<R> {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}
impl<R: Resource> Eq for ResourceRef<R> {}
impl<R: Resource> Field for ResourceRef<R> {
    fn shape() -> Shape {
        Shape::Reference {
            kind: R::KIND.into(),
        }
    }
    fn encode(&self) -> Value {
        Value::String(self.id.clone())
    }
    fn decode(value: Value) -> Result<Self> {
        match value {
            Value::String(id) => Self::new(id),
            _ => Err(Error::invalid(R::KIND, "reference identity")),
        }
    }
}
impl<T: Field> Field for Option<T> {
    fn shape() -> Shape {
        Shape::Nullable(Box::new(T::shape()))
    }
    fn encode(&self) -> Value {
        self.as_ref().map_or(Value::Null, Field::encode)
    }
    fn decode(v: Value) -> Result<Self> {
        if v.is_null() {
            Ok(None)
        } else {
            T::decode(v).map(Some)
        }
    }
}
pub trait Input: Clone + Send + Sync + 'static {
    fn encode(&self) -> Value;
    fn decode(v: Value) -> Result<Self>;
}
impl<T: Field> Input for T {
    fn encode(&self) -> Value {
        Field::encode_input(self)
    }
    fn decode(v: Value) -> Result<Self> {
        validate_shape(&T::shape(), 0, None)?;
        T::decode_input(v)
    }
}
impl Input for () {
    fn encode(&self) -> Value {
        Value::Null
    }
    fn decode(v: Value) -> Result<Self> {
        if v.is_null() {
            Ok(())
        } else {
            Err(Error::invalid("input", "$"))
        }
    }
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FieldDescriptor {
    pub name: String,
    pub shape: Shape,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Descriptor {
    pub kind: String,
    pub version: u32,
    pub fields: Vec<FieldDescriptor>,
}
pub trait Resource: Clone + Send + Sync + 'static {
    const KIND: &'static str;
    fn descriptor() -> Descriptor;
    fn normalize_field(name: &str, value: Value) -> Result<Value>;
    fn encode(&self) -> Value;
    fn decode(value: Value) -> Result<Self>;
    fn definition() -> Definition<Self> {
        Definition::new()
    }
}
#[derive(Clone)]
pub struct FieldRef<R, T> {
    pub(crate) name: &'static str,
    marker: PhantomData<fn() -> (R, T)>,
}
impl<R: Resource, T: Field> FieldRef<R, T> {
    pub fn new(name: &'static str) -> Self {
        Self {
            name,
            marker: PhantomData,
        }
    }
    pub fn equals(self, value: T) -> Query<R> {
        Query {
            spec: if value.is_present() {
                QuerySpec::equal(self.name, Field::encode(&value))
            } else {
                QuerySpec::absent(self.name)
            },
            marker: PhantomData,
        }
    }
}
#[derive(Clone)]
pub struct Query<R> {
    pub(crate) spec: QuerySpec,
    marker: PhantomData<fn() -> R>,
}
impl<R: Resource> Query<R> {
    /// Select all currently authorized rows, subject to the usual snapshot bounds.
    pub fn all() -> Self {
        Self {
            spec: QuerySpec::all(),
            marker: PhantomData,
        }
    }
    pub fn and<T: Field>(mut self, field: FieldRef<R, T>, value: T) -> Self {
        self.spec = if value.is_present() {
            self.spec.and(field.name, Field::encode(&value))
        } else {
            self.spec.and_absent(field.name)
        };
        self
    }
    pub fn after_id(mut self, id: impl Into<String>) -> Self {
        self.spec = self.spec.after_id(id);
        self
    }
    pub fn limit(mut self, limit: usize) -> Self {
        self.spec = self.spec.limit(limit);
        self
    }
    pub fn spec(&self) -> &QuerySpec {
        &self.spec
    }
}
impl<R: Resource> Default for Query<R> {
    fn default() -> Self {
        Self::all()
    }
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Intent {
    pub channel: String,
    pub payload: Value,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub delivery_version: Option<u32>,
}
impl Intent {
    pub fn new(channel: &str, payload: Value) -> Self {
        Self {
            channel: channel.into(),
            payload,
            delivery_version: None,
        }
    }
}

pub struct Action<R, I> {
    pub(crate) name: &'static str,
    function: fn(&mut R, I) -> Result<Vec<Intent>>,
}
impl<R, I> Copy for Action<R, I> {}
impl<R, I> Clone for Action<R, I> {
    fn clone(&self) -> Self {
        *self
    }
}
impl<R: Resource, I: Input> Action<R, I> {
    pub const fn new(name: &'static str, function: fn(&mut R, I) -> Result<Vec<Intent>>) -> Self {
        Self { name, function }
    }
}
pub(crate) type ErasedAction =
    Arc<dyn Fn(Value, Value) -> Result<(Value, Vec<Intent>)> + Send + Sync>;
type Policy<R> = fn(&Actor, Access, &R) -> bool;
type FieldPolicy<R> = fn(&Actor, Access, &str, &R) -> bool;
pub struct Definition<R: Resource> {
    descriptor: Descriptor,
    actions: BTreeMap<String, ErasedAction>,
    policy: Option<Policy<R>>,
    field_policy: Option<FieldPolicy<R>>,
    query_policy: Option<fn(&Actor, &str) -> bool>,
    source_owner: Option<String>,
    source_metadata_policy: Option<fn(&Actor) -> bool>,
    discovery_policy: Option<discovery::DiscoveryPolicy>,
    pub(crate) duplicate: bool,
}
impl<R: Resource> Default for Definition<R> {
    fn default() -> Self {
        Self::new()
    }
}
impl<R: Resource> Definition<R> {
    pub fn new() -> Self {
        Self {
            descriptor: R::descriptor(),
            actions: BTreeMap::new(),
            policy: None,
            field_policy: None,
            query_policy: None,
            source_owner: None,
            source_metadata_policy: None,
            discovery_policy: None,
            duplicate: false,
        }
    }
    pub fn policy(mut self, policy: Policy<R>) -> Self {
        self.policy = Some(policy);
        self
    }
    /// Explicit per-field permission. Missing field policy denies every field.
    pub fn field_policy(mut self, policy: FieldPolicy<R>) -> Self {
        self.field_policy = Some(policy);
        self
    }
    /// Authorizes predicate use before consulting any rows, including empty sets.
    pub fn query_policy(mut self, policy: fn(&Actor, &str) -> bool) -> Self {
        self.query_policy = Some(policy);
        self
    }
    /// Explicit whole-record field and predicate grant; row policy still applies.
    pub fn allow_all_fields(self) -> Self {
        self.field_policy(|_, _, _, _| true)
            .query_policy(|_, _| true)
    }
    /// Freeze whole-Resource source ownership in the accepted definition.
    pub fn source_owner(mut self, source: &str) -> Self {
        self.source_owner = Some(source.into());
        self
    }
    /// Grant protected provenance inspection separately from field reads.
    pub fn source_metadata_policy(mut self, policy: fn(&Actor) -> bool) -> Self {
        self.source_metadata_policy = Some(policy);
        self
    }
    /// Explicit metadata visibility, independent of row and operation permissions.
    /// Missing policy denies all discovery. The Resource grant is required before
    /// fields or actions can be disclosed. Callbacks run in supervised bounded I/O.
    pub fn discovery_policy<F>(mut self, policy: F) -> Self
    where
        F: Fn(&Actor, DiscoveryTarget<'_>) -> bool + Send + Sync + 'static,
    {
        self.discovery_policy = Some(Arc::new(policy));
        self
    }
    pub fn action<I: Input>(mut self, action: Action<R, I>) -> Self {
        let f: ErasedAction = Arc::new(move |state, input| {
            let mut r = R::decode(state)?;
            let i = I::decode(input).map_err(|_| Error::invalid(R::KIND, action.name))?;
            let effects = (action.function)(&mut r, i)?;
            Ok((r.encode(), effects))
        });
        self.duplicate |= self.actions.insert(action.name.into(), f).is_some();
        self
    }
}
pub(crate) trait Registered: Send + Sync {
    fn descriptor(&self) -> Descriptor;
    fn descriptor_ref(&self) -> &Descriptor;
    fn allows_discovery(&self, actor: &Actor, target: DiscoveryTarget<'_>) -> bool;
    fn actions(&self) -> &BTreeMap<String, ErasedAction>;
    fn normalize(&self, v: Value) -> Result<Value>;
    fn normalize_field(&self, name: &str, value: Value) -> Result<Value>;
    fn allows(&self, actor: &Actor, access: Access, v: &Value) -> bool;
    fn allows_field(&self, actor: &Actor, access: Access, field: &str, v: &Value) -> bool;
    fn allows_query(&self, actor: &Actor, field: &str) -> bool;
    fn source_owner(&self) -> Option<&str>;
    fn allows_source_metadata(&self, actor: &Actor) -> bool;
    fn action(&self, name: &str) -> Result<ErasedAction>;
}
impl<R: Resource> Registered for Definition<R> {
    fn descriptor_ref(&self) -> &Descriptor {
        &self.descriptor
    }
    fn allows_discovery(&self, actor: &Actor, target: DiscoveryTarget<'_>) -> bool {
        self.discovery_policy
            .as_ref()
            .is_some_and(|policy| policy(actor, target))
    }
    fn actions(&self) -> &BTreeMap<String, ErasedAction> {
        &self.actions
    }
    fn source_owner(&self) -> Option<&str> {
        self.source_owner.as_deref()
    }
    fn allows_source_metadata(&self, actor: &Actor) -> bool {
        self.source_metadata_policy.is_some_and(|p| p(actor))
    }
    fn descriptor(&self) -> Descriptor {
        self.descriptor.clone()
    }
    fn normalize_field(&self, name: &str, value: Value) -> Result<Value> {
        R::normalize_field(name, value)
    }
    fn normalize(&self, v: Value) -> Result<Value> {
        let value = R::decode(v)?.encode();
        let descriptor = &self.descriptor;
        let map = value
            .as_object()
            .ok_or_else(|| Error::invalid(R::KIND, "codec object"))?;
        if map
            .keys()
            .any(|key| !descriptor.fields.iter().any(|f| &f.name == key))
        {
            return Err(Error::invalid(R::KIND, "codec fields"));
        }
        for field in &descriptor.fields {
            if !map
                .get(&field.name)
                .map_or(matches!(field.shape, Shape::Optional(_)), |v| {
                    matches_shape(v, &field.shape)
                })
            {
                return Err(Error::invalid(R::KIND, &field.name));
            }
        }
        Ok(value)
    }
    fn allows(&self, a: &Actor, access: Access, v: &Value) -> bool {
        R::decode(v.clone())
            .ok()
            .is_some_and(|r| self.policy.is_some_and(|p| p(a, access, &r)))
    }
    fn action(&self, name: &str) -> Result<ErasedAction> {
        self.actions
            .get(name)
            .cloned()
            .ok_or_else(|| Error::invalid(R::KIND, name))
    }
    fn allows_field(&self, actor: &Actor, access: Access, field: &str, value: &Value) -> bool {
        R::decode(value.clone()).ok().is_some_and(|r| {
            self.field_policy
                .is_some_and(|p| p(actor, access, field, &r))
        })
    }
    fn allows_query(&self, actor: &Actor, field: &str) -> bool {
        self.query_policy.is_some_and(|p| p(actor, field))
    }
}

pub(crate) fn matches_shape(value: &Value, shape: &Shape) -> bool {
    match shape {
        Shape::String => value.is_string(),
        Shape::Bool => value.is_boolean(),
        Shape::U64 => value.as_u64().is_some(),
        Shape::I64 => value.as_i64().is_some(),
        Shape::F64 => value.as_f64().is_some_and(f64::is_finite),
        Shape::Nullable(inner) => value.is_null() || matches_shape(value, inner),
        Shape::Optional(inner) => matches_shape(value, inner),
        Shape::List(inner) => value
            .as_array()
            .is_some_and(|vs| vs.iter().all(|v| matches_shape(v, inner))),
        Shape::Map(inner) => value
            .as_object()
            .is_some_and(|vs| vs.values().all(|v| matches_shape(v, inner))),
        Shape::Enum(variants) => value
            .as_str()
            .is_some_and(|v| variants.iter().any(|allowed| v == allowed)),
        Shape::Reference { .. } => value.as_str().is_some_and(|v| !v.is_empty()),
    }
}

pub(crate) fn validate_shape(
    shape: &Shape,
    depth: usize,
    kinds: Option<&BTreeSet<String>>,
) -> Result<()> {
    if depth > 16 {
        return Err(Error::Unsupported("field shape nesting exceeds 16".into()));
    }
    match shape {
        Shape::Optional(inner) => {
            if depth != 0 {
                return Err(Error::Unsupported(
                    "presence is only valid on a top-level Resource field".into(),
                ));
            }
            validate_shape(inner, depth + 1, kinds)
        }
        Shape::Nullable(inner) => {
            if matches!(inner.as_ref(), Shape::Nullable(_)) {
                return Err(Error::Unsupported(
                    "nested nullable has no unambiguous codec".into(),
                ));
            }
            validate_shape(inner, depth + 1, kinds)
        }
        Shape::List(inner) | Shape::Map(inner) => validate_shape(inner, depth + 1, kinds),
        Shape::Enum(variants) => {
            let unique: BTreeSet<_> = variants.iter().collect();
            if variants.is_empty()
                || variants.len() > 256
                || unique.len() != variants.len()
                || variants.iter().any(String::is_empty)
            {
                Err(Error::Unsupported(
                    "enum requires 1..=256 distinct nonempty values".into(),
                ))
            } else {
                Ok(())
            }
        }
        Shape::Reference { kind } => {
            if kind.is_empty() || kinds.is_some_and(|ks| !ks.contains(kind)) {
                Err(Error::Unsupported(format!(
                    "unregistered reference target: {kind}"
                )))
            } else {
                Ok(())
            }
        }
        _ => Ok(()),
    }
}

#[derive(Clone, Debug)]
pub(crate) enum Mutation {
    Create(Value),
    Replace(Value),
    Patch(BTreeMap<String, FieldUpdate>),
    Delete,
    Action(String, Value),
}
#[derive(Clone, Debug)]
pub struct Command<R> {
    pub(crate) id: String,
    pub(crate) expected: Option<u64>,
    pub(crate) identity: String,
    pub(crate) mutation: Mutation,
    marker: PhantomData<fn() -> R>,
}
impl<R: Resource> Command<R> {
    pub fn create(id: &str, value: R) -> Self {
        Self {
            id: id.into(),
            expected: None,
            identity: String::new(),
            mutation: Mutation::Create(value.encode()),
            marker: PhantomData,
        }
    }
    pub fn replace(id: &str, value: R) -> Self {
        Self {
            id: id.into(),
            expected: None,
            identity: String::new(),
            mutation: Mutation::Replace(value.encode()),
            marker: PhantomData,
        }
    }
    pub fn patch(id: &str, patch: Patch<R>) -> Self {
        Self {
            id: id.into(),
            expected: None,
            identity: String::new(),
            mutation: Mutation::Patch(patch.fields),
            marker: PhantomData,
        }
    }
    pub fn delete(id: &str) -> Self {
        Self {
            id: id.into(),
            expected: None,
            identity: String::new(),
            mutation: Mutation::Delete,
            marker: PhantomData,
        }
    }
    pub fn action<I: Input>(id: &str, action: Action<R, I>, input: I) -> Self {
        Self {
            id: id.into(),
            expected: None,
            identity: String::new(),
            mutation: Mutation::Action(action.name.into(), input.encode()),
            marker: PhantomData,
        }
    }
    pub fn at_revision(mut self, revision: u64) -> Self {
        self.expected = Some(revision);
        self
    }
    pub fn idempotency(mut self, key: &str) -> Self {
        self.identity = key.into();
        self
    }
}
#[derive(Clone, Debug)]
pub struct Snapshot<R> {
    pub id: String,
    pub revision: u64,
    pub value: Option<R>,
}
pub(crate) fn typed<R: Resource>(row: Row) -> Result<Snapshot<R>> {
    Ok(Snapshot {
        id: row.key.id,
        revision: row.revision,
        value: row.value.map(R::decode).transpose()?,
    })
}
