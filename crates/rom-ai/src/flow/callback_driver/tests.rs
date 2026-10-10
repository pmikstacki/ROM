//! Exercise the shared production loop, not a copied phase runner.
use super::{PhaseFuture, drive};
use crate::{AiError, AiResult, flow::callback_phase::Step};
use std::{
    future::Future,
    pin::Pin,
    sync::{
        Arc, Mutex,
        atomic::{AtomicUsize, Ordering},
    },
    task::{Context, Poll, Waker},
};
use tokio::sync::Semaphore;

#[derive(Default)]
struct Trace {
    live: AtomicUsize,
    drops: AtomicUsize,
    events: Mutex<Vec<&'static str>>,
}
struct Probe {
    trace: Arc<Trace>,
    permits: Arc<Semaphore>,
    result: Option<AiResult<Step>>,
    name: &'static str,
}
impl Future for Probe {
    type Output = AiResult<Step>;
    fn poll(mut self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<Self::Output> {
        assert_eq!(
            self.permits.available_permits(),
            0,
            "permit cannot escape an active phase"
        );
        self.trace.events.lock().unwrap().push(self.name);
        match self.result.take() {
            Some(result) => Poll::Ready(result),
            None => Poll::Pending,
        }
    }
}
impl Drop for Probe {
    fn drop(&mut self) {
        let held = self.permits.available_permits() == 0;
        let single = self.trace.live.fetch_sub(1, Ordering::SeqCst) == 1;
        self.trace.drops.fetch_add(1, Ordering::SeqCst);
        self.trace.events.lock().unwrap().push(if held && single {
            "drop-held"
        } else {
            "invalid-drop"
        });
    }
}
fn phase(
    trace: &Arc<Trace>,
    permits: &Arc<Semaphore>,
    name: &'static str,
    result: Option<AiResult<Step>>,
) -> PhaseFuture<'static> {
    assert_eq!(
        trace.live.load(Ordering::SeqCst),
        0,
        "predecessor must drop before successor construction"
    );
    trace.live.fetch_add(1, Ordering::SeqCst);
    assert_eq!(
        permits.available_permits(),
        0,
        "one permit spans phase transitions"
    );
    trace.events.lock().unwrap().push("construct");
    Box::pin(Probe {
        trace: trace.clone(),
        permits: permits.clone(),
        result,
        name,
    })
}
fn poll_once<F: Future>(future: Pin<&mut F>) -> Poll<F::Output> {
    future.poll(&mut Context::from_waker(Waker::noop()))
}

#[test]
fn completed_phase_drops_before_successor_factory_and_releases_one_permit() {
    let permits = Arc::new(Semaphore::new(1));
    let permit = permits.clone().try_acquire_owned().unwrap();
    let trace = Arc::new(Trace::default());
    let mut factory_calls = 0;
    let mut future = Box::pin(drive(permit, Step::Continue("first".into()), |step| {
        factory_calls += 1;
        let Step::Continue(name) = step else {
            panic!("unexpected phase")
        };
        match name.as_str() {
            "first" => phase(
                &trace,
                &permits,
                "first-ready",
                Some(Ok(Step::Continue("second".into()))),
            ),
            "second" => phase(&trace, &permits, "second-ready", Some(Ok(Step::Done))),
            _ => panic!("unexpected successor"),
        }
    }));
    assert!(matches!(poll_once(future.as_mut()), Poll::Ready(Ok(()))));
    drop(future);
    assert_eq!(factory_calls, 2);
    assert_eq!(trace.drops.load(Ordering::SeqCst), 2);
    assert_eq!(trace.live.load(Ordering::SeqCst), 0);
    assert_eq!(
        *trace.events.lock().unwrap(),
        [
            "construct",
            "first-ready",
            "drop-held",
            "construct",
            "second-ready",
            "drop-held"
        ]
    );
    assert_eq!(permits.available_permits(), 1);
}

#[test]
fn failed_phase_drops_without_constructing_a_successor_or_erasing_error() {
    let permits = Arc::new(Semaphore::new(1));
    let permit = permits.clone().try_acquire_owned().unwrap();
    let trace = Arc::new(Trace::default());
    let mut calls = 0;
    let mut future = Box::pin(drive(permit, Step::Continue("failed".into()), |_| {
        calls += 1;
        phase(
            &trace,
            &permits,
            "error-ready",
            Some(Err(AiError::UnknownOutcome)),
        )
    }));
    assert!(matches!(
        poll_once(future.as_mut()),
        Poll::Ready(Err(AiError::UnknownOutcome))
    ));
    drop(future);
    assert_eq!(calls, 1);
    assert_eq!(trace.drops.load(Ordering::SeqCst), 1);
    assert_eq!(trace.live.load(Ordering::SeqCst), 0);
    assert_eq!(
        *trace.events.lock().unwrap(),
        ["construct", "error-ready", "drop-held"]
    );
    assert_eq!(permits.available_permits(), 1);
}

#[test]
fn cancellation_drops_pending_phase_once_then_releases_the_same_permit() {
    let permits = Arc::new(Semaphore::new(1));
    let permit = permits.clone().try_acquire_owned().unwrap();
    let trace = Arc::new(Trace::default());
    let mut calls = 0;
    let mut future = Box::pin(drive(permit, Step::Continue("first".into()), |step| {
        calls += 1;
        let Step::Continue(name) = step else {
            panic!("unexpected phase")
        };
        match name.as_str() {
            "first" => phase(
                &trace,
                &permits,
                "first-ready",
                Some(Ok(Step::Continue("pending".into()))),
            ),
            "pending" => phase(&trace, &permits, "pending", None),
            _ => panic!("unexpected successor"),
        }
    }));
    assert!(poll_once(future.as_mut()).is_pending());
    assert_eq!(trace.drops.load(Ordering::SeqCst), 1);
    assert_eq!(trace.live.load(Ordering::SeqCst), 1);
    assert_eq!(permits.available_permits(), 0);
    assert!(poll_once(future.as_mut()).is_pending());
    drop(future);
    assert_eq!(calls, 2, "cancellation must not construct a successor");
    assert_eq!(trace.drops.load(Ordering::SeqCst), 2);
    assert_eq!(trace.live.load(Ordering::SeqCst), 0);
    assert_eq!(
        *trace.events.lock().unwrap(),
        [
            "construct",
            "first-ready",
            "drop-held",
            "construct",
            "pending",
            "pending",
            "drop-held"
        ]
    );
    assert_eq!(permits.available_permits(), 1);
}
