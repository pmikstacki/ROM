//! Supervised reaction worker ownership and finite batch admission.
use crate::*;
pub struct ReactionWorker {
    task: tokio::task::JoinHandle<Result<()>>,
}
impl ReactionWorker {
    pub async fn join(self) -> Result<()> {
        self.task.await.map_err(|_| Error::Panicked)?
    }
}
impl Runtime {
    /// Start one generic runtime worker. Runtime shutdown stops it; dropping this observer does not.
    pub fn start_reactions(&self) -> Result<ReactionWorker> {
        self.ensure_open()?;
        let permit = execution::acquire(&self.0.reaction_worker)?;
        let lifetime = self.track_worker()?;
        let runtime = self.clone();
        let mut changes = self.0.changes.subscribe();
        let task = tokio::spawn(async move {
            let _permit = permit;
            let result=async {loop {
                if runtime.ensure_open() == Err(Error::Closed) {
                    return Ok(());
                }
                runtime.ensure_open()?;
                match runtime
                    .process_reactions((runtime.0.reaction_limits.max_work as usize).min(32))
                    .await
                {
                    Ok(0) | Err(Error::Overloaded) => {}
                    Ok(_) => continue,
                    Err(Error::Closed) => return Ok(()),
                    Err(e) => return Err(e),
                }
                tokio::select! {_ = changes.changed()=>{},_ = tokio::time::sleep(std::time::Duration::from_millis(100))=>{}}
            }}.await;
            drop(runtime);
            drop(_permit);
            drop(lifetime);
            result
        });
        Ok(ReactionWorker { task })
    }
    /// Process a finite batch on the runtime's supervised storage executor.
    /// Accepted work continues if its caller is cancelled. No per-Resource worker is needed.
    pub async fn process_reactions(&self, max_steps: usize) -> Result<usize> {
        if max_steps == 0 || max_steps > self.0.reaction_limits.max_work as usize {
            return Err(Error::TooLarge);
        }
        if !self.0.storage.supports_reactions() {
            return Err(Error::Unsupported("durable reactions".into()));
        }
        let permit = execution::acquire(&self.0.admission)?;
        self.io(move |runtime| {
            let _permit = permit;
            let mut completed = 0;
            for _ in 0..max_steps {
                runtime.ensure_open()?;
                let now = runtime.0.clock.now();
                let WorkResult::Claimed(claim) = runtime
                    .0
                    .storage
                    .reaction_update(WorkUpdate::Claim { now })?
                else {
                    break;
                };
                runtime.process_claim(*claim)?;
                completed += 1;
            }
            Ok(completed)
        })
        .await
    }
}
