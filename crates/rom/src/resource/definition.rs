//! Typed Resource definitions and their erased runtime registration contract.
use crate::{
    Access, Actor, Descriptor, DiscoveryTarget, Error, Input, Resource, Result, Value,
    canonical_fields, discovery, replay,
};
use serde::{Deserialize, Serialize};
use std::{collections::BTreeMap, sync::Arc};

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
type TransitionValidator<R> = fn(&Actor, Option<&R>, Option<&R>) -> Result<()>;
pub struct Definition<R: Resource> {
    descriptor: Descriptor,
    actions: BTreeMap<String, ErasedAction>,
    policy: Option<Policy<R>>,
    transition_validator: Option<TransitionValidator<R>>,
    field_policy: Option<FieldPolicy<R>>,
    query_policy: Option<fn(&Actor, &str) -> bool>,
    sort_policy: Option<fn(&Actor, &str) -> bool>,
    source_owner: Option<String>,
    source_metadata_policy: Option<fn(&Actor) -> bool>,
    discovery_policy: Option<discovery::DiscoveryPolicy>,
    pub(crate) duplicate: bool,
    replay_codecs: BTreeMap<u32, replay::Codec>,
    pub(crate) replay_error: Option<Error>,
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
            transition_validator: None,
            field_policy: None,
            query_policy: None,
            sort_policy: None,
            source_owner: None,
            source_metadata_policy: None,
            discovery_policy: None,
            duplicate: false,
            replay_codecs: BTreeMap::new(),
            replay_error: None,
        }
    }
    /// Retain an older Resource codec only for fingerprint checks on existing receipts.
    /// The old kind must match and its positive version must precede this definition.
    /// Replay uses the exact receipt version after current authorization; new commands
    /// never use legacy codecs. Registering a duplicate old version fails at startup.
    pub fn replay_from<Old: Resource>(mut self) -> Self {
        let codec = replay::Codec::new::<Old>();
        if let Err(error) = codec.validate_for(&self.descriptor) {
            self.replay_error = Some(error);
        }
        self.duplicate |= self
            .replay_codecs
            .insert(codec.descriptor().version, codec)
            .is_some();
        self
    }
    pub fn policy(mut self, policy: Policy<R>) -> Self {
        self.policy = Some(policy);
        self
    }
    /// Validate every proposed Resource transition after authority and revision checks.
    ///
    /// The optional previous/candidate values represent creation/deletion. Values
    /// have passed Resource codecs. This hook applies equally to built-ins and
    /// custom actions; returning an error prevents all durable effects.
    ///
    /// The callback must be pure, deterministic and bounded: it runs on supervised
    /// blocking execution under the commit gate, so it must not perform I/O or
    /// reenter the runtime. It may be evaluated again on a caller retry. Matching
    /// receipt replay does not revalidate an obsolete transition; current access
    /// rules still govern replay disclosure. A panic fails only this operation.
    pub fn validate_transition(mut self, validate: TransitionValidator<R>) -> Self {
        self.transition_validator = Some(validate);
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
    /// Authorizes ordering before consulting rows. Field-read grants are also required.
    pub fn sort_policy(mut self, policy: fn(&Actor, &str) -> bool) -> Self {
        self.sort_policy = Some(policy);
        self
    }
    /// Explicit whole-record field, predicate and sort grant; row policy still applies.
    pub fn allow_all_fields(self) -> Self {
        self.field_policy(|_, _, _, _| true)
            .query_policy(|_, _| true)
            .sort_policy(|_, _| true)
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
            let i = I::decode(input).map_err(|error| match error {
                Error::Invalid { kind, field }
                    if kind == "input" && I::field_names().contains(&field.as_str()) =>
                {
                    Error::invalid(R::KIND, &format!("{}.{}", action.name, field))
                }
                _ => Error::invalid(R::KIND, action.name),
            })?;
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
    fn replay_codec(&self, version: u32) -> Option<&replay::Codec>;
    fn allows_discovery(&self, actor: &Actor, target: DiscoveryTarget<'_>) -> bool;
    fn actions(&self) -> &BTreeMap<String, ErasedAction>;
    fn normalize(&self, v: Value) -> Result<Value>;
    fn normalize_field(&self, name: &str, value: Value) -> Result<Value>;
    fn allows(&self, actor: &Actor, access: Access, v: &Value) -> bool;
    fn allows_field(&self, actor: &Actor, access: Access, field: &str, v: &Value) -> bool;
    fn allows_query(&self, actor: &Actor, field: &str) -> bool;
    fn allows_sort(&self, actor: &Actor, field: &str) -> bool;
    fn source_owner(&self) -> Option<&str>;
    fn allows_source_metadata(&self, actor: &Actor) -> bool;
    fn action(&self, name: &str) -> Result<ErasedAction>;
    fn validate_transition(
        &self,
        actor: &Actor,
        before: Option<&Value>,
        after: Option<&Value>,
    ) -> Result<()>;
}
impl<R: Resource> Registered for Definition<R> {
    fn replay_codec(&self, version: u32) -> Option<&replay::Codec> {
        self.replay_codecs.get(&version)
    }
    fn validate_transition(
        &self,
        actor: &Actor,
        before: Option<&Value>,
        after: Option<&Value>,
    ) -> Result<()> {
        let Some(validate) = self.transition_validator else {
            return Ok(());
        };
        let before = before.cloned().map(R::decode).transpose()?;
        let after = after.cloned().map(R::decode).transpose()?;
        validate(actor, before.as_ref(), after.as_ref())
    }
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
        canonical_fields(&self.descriptor, &value)?;
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
    fn allows_sort(&self, actor: &Actor, field: &str) -> bool {
        self.sort_policy.is_some_and(|p| p(actor, field))
    }
}
