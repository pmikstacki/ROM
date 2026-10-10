//! One supervised command pipeline, including receipt replay and atomic commit.
use super::{Runtime, diagnostic_record};
use crate::{
    Access, Actor, Bundle, Cause, ClaimKey, Command, Error, FieldUpdate, Invocation, Key, Mutation,
    ProtectedMetadata, Receipt, Resource, Result, Row, Snapshot, invocation, normalize_patch,
    replay, typed,
};
use crate::{DiagnosticOutcome as Outcome, DiagnosticStage as Stage, diagnostics::OperationRecord};
use std::panic::{AssertUnwindSafe, catch_unwind};
impl Runtime {
    pub async fn execute<R: Resource>(
        &self,
        actor: &Actor,
        cmd: Command<R>,
    ) -> Result<Snapshot<R>> {
        typed(self.invoke(actor, cmd.into()).await?)
    }
    pub async fn invoke(&self, actor: &Actor, invocation: Invocation) -> Result<Row> {
        let row = self.invoke_row(actor, invocation).await?;
        let a = actor.clone();
        self.observe(actor, move |runtime| {
            let current = runtime.0.storage.load(&row.key)?;
            runtime.require_complete(&a, current.as_ref(), &row)?;
            Ok(row.clone().public_outcome())
        })
        .await
    }
    pub(crate) async fn invoke_row(&self, actor: &Actor, invocation: Invocation) -> Result<Row> {
        self.check_actor(actor)?;
        invocation.check_size(actor, self.0.limits.command_bytes)?;
        if invocation.idempotency.is_empty() || invocation.id.is_empty() {
            return Err(Error::invalid(&invocation.kind, "identity"));
        }
        let identity = invocation.durable_identity(actor);
        let cmd = invocation.into_command();
        let mut recording = self
            .0
            .diagnostics
            .as_ref()
            .map(|sink| sink.operation(&identity, None, None, 0));
        diagnostic_record::stage(&mut recording, Stage::Admission, Outcome::Started, 0);
        let permit = match self.acquire_action() {
            Ok(permit) => permit,
            Err(error) => {
                diagnostic_record::stage(
                    &mut recording,
                    Stage::Admission,
                    diagnostic_record::outcome(&error),
                    0,
                );
                diagnostic_record::publish(recording);
                return Err(error);
            }
        };
        diagnostic_record::stage(&mut recording, Stage::Admission, Outcome::Succeeded, 0);
        let a = actor.clone();
        let row = self
            .io(move |runtime| {
                let _permit = permit;
                if recording.is_some() {
                    runtime.run_observed(&a, cmd, identity, None, recording)
                } else {
                    runtime.run(&a, cmd, identity, None)
                }
            })
            .await?;
        let a = actor.clone();
        self.observe(actor, move |runtime| {
            let def = runtime
                .0
                .registry
                .get(&row.key.kind)
                .ok_or(Error::Unregistered)?;
            let current = runtime.0.storage.load(&row.key)?;
            runtime.disclose(&a, &(), def.as_ref(), current.as_ref(), &row)?;
            Ok(row.clone())
        })
        .await
    }
    pub(crate) fn run(
        &self,
        actor: &Actor,
        cmd: invocation::ErasedCommand,
        identity: String,
        causal: Option<(Cause, ClaimKey)>,
    ) -> Result<Row> {
        let recording = self.0.diagnostics.as_ref().map(|sink| {
            sink.operation(
                &identity,
                causal.as_ref().map(|(cause, _)| cause),
                causal.as_ref().map(|(_, claim)| claim),
                0,
            )
        });
        self.run_observed(actor, cmd, identity, causal, recording)
    }
    pub(crate) fn run_observed(
        &self,
        actor: &Actor,
        cmd: invocation::ErasedCommand,
        identity: String,
        causal: Option<(Cause, ClaimKey)>,
        mut recording: Option<OperationRecord>,
    ) -> Result<Row> {
        let result = self.run_inner(actor, cmd, identity, causal, &mut recording);
        if let Err(error) = &result {
            let outcome = diagnostic_record::outcome(error);
            let stage = if matches!(error, Error::Denied) {
                Stage::Authorization
            } else {
                recording
                    .as_ref()
                    .and_then(OperationRecord::last_stage)
                    .unwrap_or(Stage::Execution)
            };
            let already_recorded = recording.as_ref().is_some_and(|record| {
                record.last_stage() == Some(stage) && record.last_outcome() == Some(outcome)
            });
            if !already_recorded {
                diagnostic_record::stage(&mut recording, stage, outcome, 0);
            }
        }
        // All inner guards and native transactions have returned before queue publication.
        diagnostic_record::publish(recording);
        result
    }
    fn run_inner(
        &self,
        actor: &Actor,
        cmd: invocation::ErasedCommand,
        identity: String,
        causal: Option<(Cause, ClaimKey)>,
        recording: &mut Option<OperationRecord>,
    ) -> Result<Row> {
        // run executes in bounded I/O. Resolve current authority under the commit
        // gate before exposing registry membership or invoking application codecs.
        // Later checks still protect receipt disclosure and commit after proposal work.
        diagnostic_record::stage(recording, Stage::Authorization, Outcome::Started, 0);
        {
            let _guard = self.0.gate.lock().map_err(|_| Error::Panicked)?;
            self.check_authority(actor)?;
        }
        diagnostic_record::stage(recording, Stage::Authorization, Outcome::Succeeded, 0);
        let mut causal = causal;
        if let Some((cause, claim)) = &mut causal {
            cause.parent = Some(claim.id.clone());
        }
        diagnostic_record::stage(recording, Stage::Resolve, Outcome::Started, 0);
        let def = self.0.registry.get(&cmd.kind).ok_or(Error::Unregistered)?;
        let key = Key {
            kind: cmd.kind.clone(),
            id: cmd.id.clone(),
        };
        match (def.source_owner(), actor.source.as_ref()) {
            (None, None) => {}
            (Some(owner), Some(permit))
                if owner == permit.provenance.source && permit.target == key => {}
            _ => return Err(Error::Denied),
        }
        diagnostic_record::stage(recording, Stage::Resolve, Outcome::Succeeded, 0);
        // A retained receipt defines its original request interpretation. Check it
        // before the current codec can reject a renamed or transformed old input.
        {
            let _guard = self.0.gate.lock().map_err(|_| Error::Panicked)?;
            self.check_authority(actor)?;
            diagnostic_record::stage(recording, Stage::Receipt, Outcome::Started, 0);
            if let Some(receipt) =
                self.retry_receipt(&identity, cmd.retry_epoch, causal.is_some())?
            {
                let current = diagnostic_record::load(self.0.storage.as_ref(), &key, recording)?;
                diagnostic_record::stage(recording, Stage::Receipt, Outcome::Started, 0);
                let result =
                    self.replay_outcome(actor, def.as_ref(), current.as_ref(), receipt, &cmd);
                if result.is_ok() {
                    diagnostic_record::stage(recording, Stage::Receipt, Outcome::Replay, 0);
                }
                return result;
            }
            diagnostic_record::stage(recording, Stage::Receipt, Outcome::Succeeded, 0);
        }
        diagnostic_record::stage(recording, Stage::Validation, Outcome::Started, 0);
        let input = replay::input(def.as_ref(), &cmd.mutation)?;
        let explicit_fields = !matches!(cmd.mutation, Mutation::Action(_, _) | Mutation::Patch(_));
        let patch_fields = match &cmd.mutation {
            Mutation::Patch(fields) => Some(fields.keys().cloned().collect::<Vec<_>>()),
            _ => None,
        };
        let fingerprint = replay::fingerprint(cmd.expected, input, actor);
        let prior = {
            let _denied = self.0.gate.lock().unwrap();
            self.check_authority(actor)?;
            let prior = diagnostic_record::load(self.0.storage.as_ref(), &key, recording)?;
            diagnostic_record::stage(recording, Stage::Receipt, Outcome::Started, 0);
            if let Some(receipt) =
                self.retry_receipt(&identity, cmd.retry_epoch, causal.is_some())?
            {
                let result =
                    self.replay_outcome(actor, def.as_ref(), prior.as_ref(), receipt, &cmd);
                if result.is_ok() {
                    diagnostic_record::stage(recording, Stage::Receipt, Outcome::Replay, 0);
                }
                return result;
            }
            diagnostic_record::stage(recording, Stage::Receipt, Outcome::Succeeded, 0);
            diagnostic_record::stage(recording, Stage::Validation, Outcome::Started, 0);
            let authorization = prior
                .as_ref()
                .and_then(|r| r.value.as_ref())
                .or(match &cmd.mutation {
                    Mutation::Create(v) => Some(v),
                    _ => None,
                })
                .ok_or(Error::Missing)?;
            if !def.allows(actor, Access::Write, authorization) {
                return Err(Error::Denied);
            }
            if prior.as_ref().map(|r| r.revision) != cmd.expected {
                return Err(Error::Conflict);
            }
            prior
        };
        self.check_actor(actor)?;
        diagnostic_record::stage(recording, Stage::Execution, Outcome::Started, 0);
        // Native business functions compute proposals off Tokio. They must be pure w.r.t. external effects.
        let (new_value, effects) = match &cmd.mutation {
            Mutation::Create(v) | Mutation::Replace(v) => (Some(def.normalize(v.clone())?), vec![]),
            Mutation::Patch(fields) => {
                let mut value = prior
                    .as_ref()
                    .and_then(|r| r.value.clone())
                    .ok_or(Error::Missing)?;
                let map = value.as_object_mut().ok_or(Error::Storage)?;
                for (name, update) in normalize_patch(def.as_ref(), fields)? {
                    match update {
                        FieldUpdate::Set(v) => {
                            map.insert(name, v);
                        }
                        FieldUpdate::Remove => {
                            map.remove(&name);
                        }
                    }
                }
                (Some(def.normalize(value)?), vec![])
            }
            Mutation::Delete => (None, vec![]),
            Mutation::Action(name, input) => {
                let value = prior
                    .as_ref()
                    .and_then(|r| r.value.clone())
                    .ok_or(Error::Missing)?;
                let action = def.action(name)?;
                let input = input.clone();
                let (sender, receiver) = std::sync::mpsc::sync_channel(1);
                self.0.pool.spawn(move || {
                    let result = catch_unwind(AssertUnwindSafe(|| action(value, input)))
                        .unwrap_or(Err(Error::Panicked));
                    let _ = sender.send(result);
                });
                let (v, e) = receiver.recv().map_err(|_| Error::Panicked)??;
                (Some(def.normalize(v)?), e)
            }
        };
        if new_value
            .as_ref()
            .is_some_and(|v| !def.allows(actor, Access::Write, v))
        {
            return Err(Error::Denied);
        }
        if let Some(fields) = patch_fields.as_ref() {
            for field in fields {
                for value in [
                    prior.as_ref().and_then(|r| r.value.as_ref()),
                    new_value.as_ref(),
                ]
                .into_iter()
                .flatten()
                {
                    if !def.allows_field(actor, Access::Write, field, value) {
                        return Err(Error::Denied);
                    }
                }
            }
        }
        self.authorize_fields(
            actor,
            def.as_ref(),
            prior.as_ref().and_then(|r| r.value.as_ref()),
            new_value.as_ref(),
            explicit_fields,
        )?;
        if effects.len() > 8
            || effects
                .iter()
                .map(|e| e.payload.to_string().len() + e.channel.len())
                .sum::<usize>()
                > self.0.limits.command_bytes
            || new_value
                .as_ref()
                .is_some_and(|v| v.to_string().len() > self.0.limits.command_bytes)
        {
            return Err(Error::TooLarge);
        }
        let denied = self.0.gate.lock().unwrap();
        self.check_authority(actor)?;
        let current = diagnostic_record::load(self.0.storage.as_ref(), &key, recording)?;
        diagnostic_record::stage(recording, Stage::Receipt, Outcome::Started, 0);
        if let Some(receipt) = self.retry_receipt(&identity, cmd.retry_epoch, causal.is_some())? {
            let result = self.replay_outcome(actor, def.as_ref(), current.as_ref(), receipt, &cmd);
            if result.is_ok() {
                diagnostic_record::stage(recording, Stage::Receipt, Outcome::Replay, 0);
            }
            return result;
        }
        diagnostic_record::stage(recording, Stage::Receipt, Outcome::Succeeded, 0);
        diagnostic_record::stage(recording, Stage::Validation, Outcome::Started, 0);
        // Recheck authoritative current state after CPU work and before conditional commit.
        if let Some(v) = current.as_ref().and_then(|r| r.value.as_ref())
            && !def.allows(actor, Access::Write, v)
        {
            return Err(Error::Denied);
        }
        if current.as_ref().map(|r| r.revision) != cmd.expected {
            return Err(Error::Conflict);
        }
        self.authorize_fields(
            actor,
            def.as_ref(),
            current.as_ref().and_then(|r| r.value.as_ref()),
            new_value.as_ref(),
            explicit_fields,
        )?;
        // Validate the actual CAS-checked transition for every mutation path.
        // Catch locally: application failure must not unwind through/poison the gate.
        diagnostic_record::stage(recording, Stage::Validation, Outcome::Started, 0);
        catch_unwind(AssertUnwindSafe(|| {
            def.validate_transition(
                actor,
                current.as_ref().and_then(|r| r.value.as_ref()),
                new_value.as_ref(),
            )
        }))
        .unwrap_or(Err(Error::Panicked))?;
        diagnostic_record::stage(recording, Stage::Validation, Outcome::Succeeded, 0);
        let source_provenance = actor
            .source
            .as_ref()
            .map(|permit| permit.provenance.clone())
            .or_else(|| {
                current
                    .as_ref()
                    .and_then(|row| row.protected.source_provenance.clone())
            });
        if let (Some(permit), Some(value)) = (&actor.source, &new_value) {
            let fields = value
                .as_object()
                .ok_or_else(|| Error::invalid(&cmd.kind, "fields"))?;
            if fields.len() != permit.provenance.field_origins.len()
                || fields
                    .keys()
                    .any(|key| !permit.provenance.field_origins.contains_key(key))
            {
                return Err(Error::invalid(&cmd.kind, "source origins"));
            }
        }
        let changed = current.as_ref().and_then(|r| r.value.as_ref()) != new_value.as_ref()
            || current
                .as_ref()
                .and_then(|r| r.protected.source_provenance.as_ref())
                != source_provenance.as_ref();
        let revision = cmd
            .expected
            .unwrap_or(0)
            .checked_add(u64::from(changed))
            .ok_or(Error::TooLarge)?;
        let row = Row {
            key,
            revision,
            protected: ProtectedMetadata {
                source_provenance,
                deletion_authorization: if new_value.is_none() {
                    current.as_ref().and_then(|r| r.value.clone())
                } else {
                    None
                },
            },
            value: new_value,
        };
        // No-op effects are rejected: an external effect needs a committed transition in this slice.
        if !changed && !effects.is_empty() {
            return Err(Error::invalid(&cmd.kind, "no-op effects"));
        }
        let mut reactions = if changed {
            self.reaction_intents(
                current.as_ref(),
                &row,
                &identity,
                cmd.retry_epoch,
                causal.as_ref().map(|(cause, _)| cause),
            )?
        } else {
            vec![]
        };
        reactions.extend(self.notification_intents(
            &row,
            &effects,
            &identity,
            cmd.retry_epoch,
            causal.as_ref().map(|(cause, _)| cause),
        )?);
        let bundle = Bundle {
            reactions,
            reaction_limits: Some(self.0.reaction_limits.clone()),
            completed_work: causal.map(|(_, claim)| (claim, self.0.clock.now())),
            expected: cmd.expected,
            receipt: Receipt {
                retry_epoch: cmd.retry_epoch,
                replay_version: Some(def.descriptor_ref().version),
                identity,
                fingerprint,
                row: row.clone(),
            },
            changed,
            effects,
        };
        self.check_authority(actor)?;
        diagnostic_record::stage(recording, Stage::Commit, Outcome::Started, 0);
        let receipt = self.0.storage.commit(&bundle);
        diagnostic_record::stage(
            recording,
            Stage::Commit,
            match &receipt {
                Ok(_) => Outcome::Succeeded,
                Err(error) => diagnostic_record::outcome(error),
            },
            0,
        );
        // Unknown may mean committed: invalidate even when the adapter loses its acknowledgment.
        if receipt.is_ok() || receipt == Err(Error::Unknown) {
            self.invalidate();
        }
        let receipt = receipt?;
        if changed {
            diagnostic_record::stage(recording, Stage::Event, Outcome::Succeeded, 1);
        }
        self.disclose(
            actor,
            &denied,
            def.as_ref(),
            Some(&receipt.row),
            &receipt.row,
        )?;
        diagnostic_record::stage(recording, Stage::Execution, Outcome::Succeeded, 0);
        Ok(receipt.row)
    }
}
