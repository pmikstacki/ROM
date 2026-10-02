use super::*;
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Shape {
    String,
    Bool,
    U64,
    Nullable(Box<Shape>),
}
pub trait Field: Clone + Send + Sync + 'static {
    fn shape() -> Shape;
    fn encode(&self) -> Value;
    fn decode(value: Value) -> Result<Self>;
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
        Field::encode(self)
    }
    fn decode(v: Value) -> Result<Self> {
        T::decode(v)
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
            field: self.name.into(),
            value: Field::encode(&value),
            marker: PhantomData,
        }
    }
}
#[derive(Clone)]
pub struct Query<R> {
    pub(crate) field: String,
    pub(crate) value: Value,
    marker: PhantomData<fn() -> R>,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Actor {
    pub authority: String,
    pub subject: String,
}
impl Actor {
    /// Trusted host construction; not a credential verifier.
    pub fn trusted(authority: &str, subject: &str) -> Self {
        Self {
            authority: authority.into(),
            subject: subject.into(),
        }
    }
    pub(crate) fn key(&self) -> String {
        serde_json::to_string(self).unwrap()
    }
}
#[derive(Clone, Copy, Debug)]
pub enum Access {
    Read,
    Write,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Intent {
    pub channel: String,
    pub payload: Value,
}
impl Intent {
    pub fn new(channel: &str, payload: Value) -> Self {
        Self {
            channel: channel.into(),
            payload,
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
pub struct Definition<R: Resource> {
    descriptor: Descriptor,
    actions: BTreeMap<String, ErasedAction>,
    policy: Option<Policy<R>>,
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
            duplicate: false,
        }
    }
    pub fn policy(mut self, policy: Policy<R>) -> Self {
        self.policy = Some(policy);
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
    fn normalize(&self, v: Value) -> Result<Value>;
    fn allows(&self, actor: &Actor, access: Access, v: &Value) -> bool;
    fn action(&self, name: &str) -> Result<ErasedAction>;
}
impl<R: Resource> Registered for Definition<R> {
    fn descriptor(&self) -> Descriptor {
        self.descriptor.clone()
    }
    fn normalize(&self, v: Value) -> Result<Value> {
        let value = R::decode(v)?.encode();
        let descriptor = &self.descriptor;
        let map = value
            .as_object()
            .ok_or_else(|| Error::invalid(R::KIND, "codec object"))?;
        if map.len() != descriptor.fields.len() {
            return Err(Error::invalid(R::KIND, "codec fields"));
        }
        for field in &descriptor.fields {
            if !map
                .get(&field.name)
                .is_some_and(|v| matches_shape(v, &field.shape))
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
}

pub(crate) fn matches_shape(value: &Value, shape: &Shape) -> bool {
    match shape {
        Shape::String => value.is_string(),
        Shape::Bool => value.is_boolean(),
        Shape::U64 => value.as_u64().is_some(),
        Shape::Nullable(inner) => value.is_null() || matches_shape(value, inner),
    }
}

#[derive(Clone, Debug)]
pub(crate) enum Mutation {
    Create(Value),
    Replace(Value),
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
