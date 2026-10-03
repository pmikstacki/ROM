//! Typed reaction declarations and erased mapper registration.
use crate::*;
/// A custom action input for an existing target Resource.
pub struct Target<I> {
    pub id: String,
    pub input: I,
}
impl<I> Target<I> {
    pub fn new(id: impl Into<String>, input: I) -> Self {
        Self {
            id: id.into(),
            input,
        }
    }
}
type Mapper = Arc<dyn Fn(Row) -> Result<Vec<(String, Value)>> + Send + Sync>;
pub(crate) struct RegisteredReaction {
    pub name: String,
    pub version: u32,
    pub source: String,
    pub target: String,
    pub action: String,
    pub actor: Actor,
    pub dependencies: BTreeSet<String>,
    pub mapper: Mapper,
}
pub struct Reaction<S, T, I> {
    inner: RegisteredReaction,
    marker: PhantomData<fn(S, T, I)>,
}
impl<S: Resource, T: Resource, I: Input> Reaction<S, T, I> {
    pub fn new(
        name: &str,
        version: u32,
        actor: Actor,
        action: Action<T, I>,
        map: fn(&Snapshot<S>) -> Result<Vec<Target<I>>>,
    ) -> Self {
        Self {
            inner: RegisteredReaction {
                name: name.into(),
                version,
                source: S::KIND.into(),
                target: T::KIND.into(),
                action: action.name.into(),
                actor,
                dependencies: BTreeSet::new(),
                mapper: Arc::new(move |row| {
                    map(&typed::<S>(row)?).map(|targets| {
                        targets
                            .into_iter()
                            .map(|t| (t.id, I::encode(&t.input)))
                            .collect()
                    })
                }),
            },
            marker: PhantomData,
        }
    }
    /// Omit dependencies for conservative routing on every semantic source change.
    pub fn depends_on<F: Field>(mut self, field: FieldRef<S, F>) -> Self {
        self.inner.dependencies.insert(field.name.into());
        self
    }
    pub(crate) fn erase(self) -> RegisteredReaction {
        self.inner
    }
}
