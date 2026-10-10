//! Stage guards exist only with test-support; disabled observations do not read a clock.
use super::{StageObservation, StageOperation, StorageStage, WorkCommitContext};
use std::time::Instant;

pub(crate) struct Timer<'a> {
    work: Option<WorkCommitContext>,
    started: Option<(&'a StageObservation, StageOperation, StorageStage, Instant)>,
}
impl<'a> Timer<'a> {
    #[cfg(test)]
    pub(super) fn started_for_test_is_none(&self) -> bool {
        self.started.is_none()
    }

    pub(crate) fn new(
        observation: Option<&'a StageObservation>,
        operation: StageOperation,
        stage: StorageStage,
    ) -> Self {
        Self {
            work: None,
            started: observation.map(|handle| (handle, operation, stage, Instant::now())),
        }
    }
    pub(crate) fn work_commit(
        observation: Option<&'a StageObservation>,
        context: Option<WorkCommitContext>,
    ) -> Self {
        let mut timer = Self::new(
            observation,
            StageOperation::WorkUpdate,
            StorageStage::NativeCommit,
        );
        timer.work = context;
        timer
    }
    pub(crate) fn finish(mut self, native_failed: bool) {
        if let Some((handle, operation, stage, start)) = self.started.take() {
            let elapsed = start.elapsed();
            handle.record(operation, stage, elapsed);
            if let Some(context) = self.work.take() {
                handle.record_work_commit(context, native_failed, elapsed);
            }
        }
    }
}
impl Drop for Timer<'_> {
    fn drop(&mut self) {
        if let Some((handle, operation, stage, start)) = self.started.take() {
            handle.record(operation, stage, start.elapsed());
            if self.work.take().is_some() {
                handle.unavailable_work_commit();
            }
        }
    }
}
