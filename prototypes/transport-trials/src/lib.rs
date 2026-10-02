//! Disposable transport experiment. Memory-only outcomes are NOT durable acceptance.
use std::{
    collections::BTreeMap,
    future::Future,
    pin::Pin,
    sync::{Arc, Condvar, Mutex},
    task::{Context, Poll},
};
use tokio::sync::{Notify, OwnedSemaphorePermit, Semaphore, mpsc, oneshot, watch};
use tokio_util::task::TaskTracker;
use tower::Service;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Error {
    Denied,
    Conflict,
    Overloaded,
    Closed,
    Unsupported,
    Gap,
    Lagged,
    Unresolved,
    TooLarge,
}
impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self:?}")
    }
}
impl std::error::Error for Error {}

/// Host-issued context, separate from decoded input; not a serialized credential.
#[derive(Clone, Copy)]
pub struct TrustedContext {
    principal: u64,
}
pub fn host_context(principal: u64) -> TrustedContext {
    TrustedContext { principal }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Resource {
    pub revision: u64,
    pub value: u64,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Command {
    pub key: u64,
    pub expected: u64,
    pub value: u64,
    pub claimed_actor: u64,
}
impl Command {
    fn semantics(self) -> (u64, u64) {
        (self.expected, self.value)
    }
}
/// Artificial CPU gate, solely to expose lifetime races deterministically.
#[derive(Default)]
pub struct Gate {
    open: Mutex<bool>,
    cv: Condvar,
    started: Notify,
}
impl Gate {
    pub fn release(&self) {
        *self.open.lock().unwrap() = true;
        self.cv.notify_all();
    }
    pub async fn started(&self) {
        self.started.notified().await;
    }
    fn enter(&self) {
        self.started.notify_one();
        let mut open = self.open.lock().unwrap();
        while !*open {
            open = self.cv.wait(open).unwrap();
        }
    }
}
#[derive(Clone)]
pub struct Attempt {
    pub context: TrustedContext,
    pub command: Command,
    pub gate: Option<Arc<Gate>>,
    pub bytes: u32,
}
type Outcome = Result<Resource, Error>;
pub type Reply = Pin<Box<dyn Future<Output = Outcome> + Send>>;
struct Entry {
    semantics: (u64, u64),
    outcome: watch::Sender<Option<Outcome>>,
}
struct State {
    closed: bool,
    allowed: bool,
    resource: Resource,
    outcomes: BTreeMap<(u64, u64), Entry>,
    journal: std::collections::VecDeque<Resource>,
    streams: Vec<Sink>,
}
#[derive(Clone)]
pub struct Core {
    state: Arc<Mutex<State>>,
    work: Arc<Semaphore>,
    bytes: Arc<Semaphore>,
    streams: Arc<Semaphore>,
    pool: Arc<rayon::ThreadPool>,
    tasks: TaskTracker,
}
impl Core {
    pub fn new(pool: Arc<rayon::ThreadPool>) -> Self {
        Self {
            state: Arc::new(Mutex::new(State {
                closed: false,
                allowed: true,
                resource: Resource {
                    revision: 0,
                    value: 0,
                },
                outcomes: BTreeMap::new(),
                journal: Default::default(),
                streams: vec![],
            })),
            work: Arc::new(Semaphore::new(2)),
            bytes: Arc::new(Semaphore::new(128)),
            streams: Arc::new(Semaphore::new(2)),
            pool,
            tasks: TaskTracker::new(),
        }
    }
    fn authorized(state: &State, context: TrustedContext) -> bool {
        state.allowed && context.principal == 1
    }
    pub fn revoke(&self) {
        self.state.lock().unwrap().allowed = false;
    }
    pub fn resource(&self) -> Resource {
        self.state.lock().unwrap().resource
    }
    pub fn available(&self) -> usize {
        self.work.available_permits()
    }
    pub fn retained_jobs(&self) -> usize {
        self.tasks.len()
    }
    pub fn stream_capacity(&self) -> usize {
        self.streams.available_permits()
    }
    fn reply(&self, context: TrustedContext, mut rx: watch::Receiver<Option<Outcome>>) -> Reply {
        let core = self.clone();
        Box::pin(async move {
            loop {
                let outcome = *rx.borrow_and_update();
                if let Some(outcome) = outcome {
                    let state = core.state.lock().unwrap();
                    return if Self::authorized(&state, context) {
                        outcome
                    } else {
                        Err(Error::Denied)
                    };
                }
                rx.changed().await.map_err(|_| Error::Unresolved)?;
            }
        })
    }
    /// Bounded synchronous admission; accepted work belongs to ROM, reply to caller.
    pub fn invoke(&self, attempt: Attempt) -> Result<Reply, Error> {
        let mut state = self.state.lock().unwrap();
        if state.closed {
            return Err(Error::Closed);
        }
        if !Self::authorized(&state, attempt.context) {
            return Err(Error::Denied);
        }
        if attempt.bytes > 128 {
            return Err(Error::TooLarge);
        }
        let key = (attempt.context.principal, attempt.command.key);
        if let Some(entry) = state.outcomes.get(&key) {
            if entry.semantics != attempt.command.semantics() {
                return Err(Error::Conflict);
            }
            return Ok(self.reply(attempt.context, entry.outcome.subscribe()));
        }
        // This finite probe rejects when its fixed-size outcome table is full.
        if state.outcomes.len() >= 16 {
            return Err(Error::Overloaded);
        }
        let work = self
            .work
            .clone()
            .try_acquire_owned()
            .map_err(|_| Error::Overloaded)?;
        let bytes = self
            .bytes
            .clone()
            .try_acquire_many_owned(attempt.bytes)
            .map_err(|_| Error::Overloaded)?;
        let (tx, rx) = watch::channel(None);
        state.outcomes.insert(
            key,
            Entry {
                semantics: attempt.command.semantics(),
                outcome: tx.clone(),
            },
        );
        let core = self.clone();
        // Registration and intake close share the state lock. TaskTracker::close alone
        // does NOT prevent later spawns; ROM supplies that exclusion.
        self.tasks.spawn(async move {
            let _permits = (work, bytes);
            let (done, result) = oneshot::channel();
            core.pool.spawn(move || {
                if let Some(gate) = attempt.gate {
                    gate.enter();
                }
                // Rayon only proposes. Resource mutation remains in the supervisor.
                let _ = done.send(attempt.command.value);
            });
            let proposal = result.await.map_err(|_| Error::Unresolved);
            let mut state = core.state.lock().unwrap();
            let outcome = if !Self::authorized(&state, attempt.context) {
                Err(Error::Denied)
            } else if state.resource.revision != attempt.command.expected {
                Err(Error::Conflict)
            } else {
                proposal.map(|value| {
                    state.resource = Resource {
                        revision: state.resource.revision + 1,
                        value,
                    };
                    let frame = state.resource;
                    state.journal.push_back(frame);
                    if state.journal.len() > 2 {
                        state.journal.pop_front();
                    }
                    state.streams.retain(|sink| !sink.closed());
                    for sink in &state.streams {
                        sink.publish(frame);
                    }
                    frame
                })
            };
            tx.send_replace(Some(outcome));
        });
        Ok(self.reply(attempt.context, rx))
    }
    pub fn begin_shutdown(&self) {
        let mut state = self.state.lock().unwrap();
        state.closed = true;
        for sink in &state.streams {
            sink.stop(Error::Closed);
        }
        state.streams.clear();
        self.tasks.close();
    }
    pub async fn drain(&self) {
        self.tasks.wait().await;
    }
    pub fn observe(&self, context: TrustedContext) -> Result<Live, Error> {
        let mut state = self.state.lock().unwrap();
        self.check_stream(&state, context)?;
        let permit = self
            .streams
            .clone()
            .try_acquire_owned()
            .map_err(|_| Error::Overloaded)?;
        let (tx, rx) = watch::channel(state.resource);
        let (stop, terminal) = watch::channel(None);
        state.streams.retain(|sink| !sink.closed());
        state.streams.push(Sink::Live(tx, stop));
        Ok(Live {
            core: self.clone(),
            context,
            rx,
            terminal,
            first: true,
            _permit: permit,
        })
    }
    pub fn subscribe(&self, context: TrustedContext, after: u64) -> Result<Journal, Error> {
        let mut state = self.state.lock().unwrap();
        self.check_stream(&state, context)?;
        if after > state.resource.revision
            || state
                .journal
                .front()
                .is_some_and(|r| after + 1 < r.revision)
        {
            return Err(Error::Gap);
        }
        let permit = self
            .streams
            .clone()
            .try_acquire_owned()
            .map_err(|_| Error::Overloaded)?;
        let (tx, rx) = mpsc::channel(2);
        for frame in &state.journal {
            if frame.revision > after {
                tx.try_send(*frame).unwrap();
            }
        }
        let (stop, terminal) = watch::channel(None);
        state.streams.retain(|sink| !sink.closed());
        state.streams.push(Sink::Journal(tx, stop));
        Ok(Journal {
            core: self.clone(),
            context,
            rx,
            terminal,
            _permit: permit,
        })
    }
    fn check_stream(&self, state: &State, context: TrustedContext) -> Result<(), Error> {
        if state.closed {
            Err(Error::Closed)
        } else if !Self::authorized(state, context) {
            Err(Error::Denied)
        } else {
            Ok(())
        }
    }
    fn disclosure(&self, context: TrustedContext) -> Result<(), Error> {
        if Self::authorized(&self.state.lock().unwrap(), context) {
            Ok(())
        } else {
            Err(Error::Denied)
        }
    }
}
enum Sink {
    Live(watch::Sender<Resource>, watch::Sender<Option<Error>>),
    Journal(mpsc::Sender<Resource>, watch::Sender<Option<Error>>),
}
impl Sink {
    fn closed(&self) -> bool {
        match self {
            Self::Live(tx, _) => tx.is_closed(),
            Self::Journal(tx, stop) => tx.is_closed() || stop.borrow().is_some(),
        }
    }
    fn stop(&self, error: Error) {
        match self {
            Self::Live(_, s) | Self::Journal(_, s) => {
                s.send_replace(Some(error));
            }
        }
    }
    fn publish(&self, frame: Resource) {
        match self {
            Self::Live(tx, _) => {
                tx.send_replace(frame);
            }
            Self::Journal(tx, _) => {
                if tx.try_send(frame).is_err() {
                    self.stop(Error::Lagged);
                }
            }
        }
    }
}
pub struct Live {
    core: Core,
    context: TrustedContext,
    rx: watch::Receiver<Resource>,
    terminal: watch::Receiver<Option<Error>>,
    first: bool,
    _permit: OwnedSemaphorePermit,
}
impl Live {
    pub async fn next(&mut self) -> Outcome {
        if let Some(e) = *self.terminal.borrow() {
            return Err(e);
        }
        if !self.first {
            tokio::select! { biased;
                _ = self.terminal.changed() => {},
                _ = self.rx.changed() => {},
            }
        }
        self.first = false;
        if let Some(e) = *self.terminal.borrow() {
            return Err(e);
        }
        self.core.disclosure(self.context)?;
        Ok(*self.rx.borrow_and_update())
    }
}
pub struct Journal {
    core: Core,
    context: TrustedContext,
    rx: mpsc::Receiver<Resource>,
    terminal: watch::Receiver<Option<Error>>,
    _permit: OwnedSemaphorePermit,
}
impl Journal {
    pub async fn next(&mut self) -> Outcome {
        if let Some(e) = *self.terminal.borrow() {
            return Err(e);
        }
        let frame = tokio::select! {biased;
            _ = self.terminal.changed() => return Err(self.terminal.borrow().unwrap_or(Error::Closed)),
            frame = self.rx.recv() => frame.ok_or(Error::Closed)?,
        };
        if let Some(e) = *self.terminal.borrow() {
            return Err(e);
        }
        self.core.disclosure(self.context)?;
        Ok(frame)
    }
    pub fn queued(&self) -> usize {
        self.rx.len()
    }
}
/// Tower compatibility is optional and admits through exactly the focused contract.
#[derive(Clone)]
pub struct TowerInvoke(pub Core);
impl Service<Attempt> for TowerInvoke {
    type Response = Resource;
    type Error = Error;
    type Future = Reply;
    // ROM's admission is request-dependent and may reject (bytes, key, auth).
    // Ready here promises ability to *attempt admission*, never accepted work.
    fn poll_ready(&mut self, _: &mut Context<'_>) -> Poll<Result<(), Error>> {
        Poll::Ready(Ok(()))
    }
    fn call(&mut self, attempt: Attempt) -> Reply {
        match self.0.invoke(attempt) {
            Ok(reply) => reply,
            Err(e) => Box::pin(async move { Err(e) }),
        }
    }
}
#[derive(Clone, Copy)]
pub struct Profile {
    pub actions: bool,
    pub live: bool,
    pub journal: bool,
}
impl Profile {
    pub fn require(self, needed: Self) -> Result<(), Error> {
        if (needed.actions && !self.actions)
            || (needed.live && !self.live)
            || (needed.journal && !self.journal)
        {
            Err(Error::Unsupported)
        } else {
            Ok(())
        }
    }
}

#[cfg(test)]
mod tests;
