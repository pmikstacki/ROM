//! Explicit retry identity boundaries, independent of schema and journal versions.
use crate::*;

pub(crate) fn is_zero(epoch: &u64) -> bool {
    *epoch == 0
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct RetryEpochs {
    pub current: u64,
    pub admission_floor: u64,
    pub replay_floor: u64,
}
impl RetryEpochs {
    pub fn validate(&self) -> Result<()> {
        if self.replay_floor > self.admission_floor || self.admission_floor > self.current {
            return Err(Error::invalid("retry", "epoch floors"));
        }
        Ok(())
    }
    /// `causal` must mean a verified existing work claim in this exact epoch.
    /// It permits completion below the admission floor, never below the replay floor.
    pub fn check(&self, epoch: u64, replay_exists: bool, causal: bool) -> Result<()> {
        self.validate()?;
        if epoch > self.current {
            return Err(Error::invalid("retry", "retry epoch"));
        }
        if epoch < self.replay_floor || (!replay_exists && !causal && epoch < self.admission_floor)
        {
            return Err(Error::IdentityExpired);
        }
        Ok(())
    }
    pub(crate) fn check_fence(&self, fence: Self) -> Result<()> {
        self.validate()?;
        fence.validate()?;
        if self.current < fence.current
            || self.admission_floor < fence.admission_floor
            || self.replay_floor < fence.replay_floor
        {
            return Err(Error::Unsupported(
                "persisted retry epochs precede trusted fence".into(),
            ));
        }
        Ok(())
    }
}

impl Runtime {
    /// Read retry boundaries under current actor authority. Requests still choose an
    /// explicit epoch; this query never changes or renews an existing request identity.
    pub async fn retry_epochs(&self, actor: &Actor) -> Result<RetryEpochs> {
        self.observe(actor, |runtime| {
            let epochs = runtime.0.storage.retry_epochs()?;
            epochs.validate()?;
            Ok(epochs)
        })
        .await
    }
    /// Called under the runtime gate before any request codec is invoked.
    pub(crate) fn retry_receipt(
        &self,
        identity: &str,
        epoch: u64,
        causal: bool,
    ) -> Result<Option<Receipt>> {
        let epochs = self.0.storage.retry_epochs()?;
        epochs.check(epoch, true, false)?;
        let receipt = self.0.storage.receipt(identity)?;
        epochs.check(epoch, receipt.is_some(), causal)?;
        if receipt
            .as_ref()
            .is_some_and(|receipt| receipt.retry_epoch != epoch)
        {
            return Err(Error::Storage);
        }
        Ok(receipt)
    }
}
