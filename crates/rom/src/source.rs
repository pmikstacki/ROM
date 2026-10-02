use super::*;

/// Protected, non-secret attribution for a complete externally owned Resource.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SourceProvenance {
    pub source: String,
    pub version: String,
    pub generation: u64,
    pub field_origins: BTreeMap<String, String>,
}
/// Authorized inspection of accepted source state, distinct from requested generation.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct SourceState {
    pub revision: u64,
    pub present: bool,
    pub provenance: Option<SourceProvenance>,
}
/// Generic Resource revision dependency, independent of a configuration loader.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct RevisionCondition {
    pub key: Key,
    pub revision: u64,
}
/// Trusted native host permit. It has no deserializer and grants no row/field access.
#[derive(Clone, PartialEq, Eq, Serialize)]
pub struct SourcePermit {
    pub(crate) target: Key,
    pub(crate) provenance: SourceProvenance,
    pub(crate) condition: RevisionCondition,
    pub(crate) valid_until: u64,
}
fn safe_label(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 128
        && value
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b"_.:-".contains(&b))
}
impl SourcePermit {
    /// Called only by a trusted host adapter after checking its managed source
    /// grant. The referenced revision is rechecked under the runtime commit gate.
    pub fn trusted(
        target: Key,
        provenance: SourceProvenance,
        condition: RevisionCondition,
        valid_until: u64,
    ) -> Result<Self> {
        if !safe_label(&provenance.source)
            || !safe_label(&provenance.version)
            || provenance.generation == 0
            || condition.revision == 0
            || valid_until == 0
            || [
                &target.kind,
                &target.id,
                &condition.key.kind,
                &condition.key.id,
            ]
            .iter()
            .any(|v| v.is_empty() || v.len() > 2048)
            || provenance.field_origins.len() > 128
            || provenance
                .field_origins
                .iter()
                .any(|(k, v)| k.is_empty() || k.len() > 256 || !safe_label(v))
        {
            return Err(Error::invalid("source", "permit"));
        }
        Ok(Self {
            target,
            provenance,
            condition,
            valid_until,
        })
    }
}
impl Runtime {
    /// Native adapter entrypoint, using the same mutation pipeline and policies.
    /// The permit is never accepted as a field in serialized Invocation.
    pub async fn invoke_sourced(
        &self,
        actor: &Actor,
        invocation: Invocation,
        permit: SourcePermit,
    ) -> Result<ProjectedView> {
        if actor.principal_kind() != PrincipalKind::Service {
            return Err(Error::Denied);
        }
        let mut a = actor.clone();
        a.source = Some(permit);
        let row = self.invoke_row(&a, invocation).await?;
        let b = a.clone();
        self.observe(&a, move |runtime| {
            let current = runtime.0.storage.load(&row.key)?;
            runtime.project_outcome(&b, current.as_ref(), &row)
        })
        .await
    }
    /// Protected metadata inspection. Whole-record field grants do not imply it.
    pub async fn source_provenance(
        &self,
        actor: &Actor,
        kind: &str,
        id: &str,
    ) -> Result<Option<SourceProvenance>> {
        Ok(self
            .source_state(actor, kind, id)
            .await?
            .and_then(|state| state.provenance))
    }
    /// Includes tombstone revisions under their retained current authorization.
    pub async fn source_state(
        &self,
        actor: &Actor,
        kind: &str,
        id: &str,
    ) -> Result<Option<SourceState>> {
        let key = Key {
            kind: kind.into(),
            id: id.into(),
        };
        let a = actor.clone();
        self.observe(actor, move |runtime| {
            let def = runtime
                .0
                .registry
                .get(&key.kind)
                .ok_or(Error::Unregistered)?;
            if !def.allows_source_metadata(&a) {
                return Err(Error::Denied);
            }
            let Some(row) = runtime.0.storage.load(&key)? else {
                return Ok(None);
            };
            runtime.disclose(&a, &(), def.as_ref(), Some(&row), &row)?;
            Ok(Some(SourceState {
                revision: row.revision,
                present: row.value.is_some(),
                provenance: row.protected.source_provenance,
            }))
        })
        .await
    }
}
