//! Typed commands and decoded Resource results.
use crate::{Action, FieldUpdate, Input, Patch, Resource, Result, Row, Value};
use std::{collections::BTreeMap, marker::PhantomData};

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
    pub(crate) retry_epoch: u64,
    pub(crate) id: String,
    pub(crate) expected: Option<u64>,
    pub(crate) identity: String,
    pub(crate) mutation: Mutation,
    marker: PhantomData<fn() -> R>,
}
impl<R: Resource> Command<R> {
    fn new(id: &str, mutation: Mutation) -> Self {
        Self {
            id: id.into(),
            expected: None,
            retry_epoch: 0,
            identity: String::new(),
            mutation,
            marker: PhantomData,
        }
    }
    pub fn create(id: &str, value: R) -> Self {
        Self::new(id, Mutation::Create(value.encode()))
    }
    pub fn replace(id: &str, value: R) -> Self {
        Self::new(id, Mutation::Replace(value.encode()))
    }
    pub fn patch(id: &str, patch: Patch<R>) -> Self {
        Self::new(id, Mutation::Patch(patch.fields))
    }
    pub fn delete(id: &str) -> Self {
        Self::new(id, Mutation::Delete)
    }
    pub fn action<I: Input>(id: &str, action: Action<R, I>, input: I) -> Self {
        Self::new(id, Mutation::Action(action.name.into(), input.encode()))
    }
    pub fn at_revision(mut self, revision: u64) -> Self {
        self.expected = Some(revision);
        self
    }
    pub fn idempotency(mut self, key: &str) -> Self {
        self.identity = key.into();
        self
    }
    /// Select the original request's epoch. Defaults to zero, never the current epoch.
    pub fn retry_epoch(mut self, epoch: u64) -> Self {
        self.retry_epoch = epoch;
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
