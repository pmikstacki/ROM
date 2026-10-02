use super::*;
fn setup(mode: Mode, bound: usize, capacity: usize) -> (tempfile::TempDir, Runtime) {
    let d = tempfile::tempdir().unwrap();
    let r = Runtime::new(
        &d.path().join("PROTOTYPE.sqlite"),
        mode,
        bound,
        capacity,
        60_000,
    );
    (d, r)
}
fn gate(r: &Runtime) -> Arc<Gate> {
    let g = Arc::new(Gate::new());
    *r.shared.gate.lock().unwrap() = Some(g.clone());
    g
}
async fn take(s: &Semaphore, n: u32) {
    tokio::time::timeout(Duration::from_secs(10), s.acquire_many(n))
        .await
        .unwrap()
        .unwrap()
        .forget();
}
fn spawn(r: &Runtime, k: Key, i: Input) -> tokio::task::JoinHandle<Reply> {
    let r = r.clone();
    tokio::spawn(async move { r.call(k, i).await })
}
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn concurrent_duplicates_have_one_commit_and_identical_outcomes_all_modes() {
    for mode in Mode::ALL {
        let (_d, r) = setup(mode, 16, 32);
        let g = gate(&r);
        let owner = spawn(&r, key(1), input(10));
        take(&g.entered, 1).await;
        let waiters: Vec<_> = (0..7).map(|_| spawn(&r, key(1), input(10))).collect();
        if mode.flight() {
            take(&g.joined, 7).await;
        } else {
            take(&g.entered, 7).await;
        }
        g.release.add_permits(8);
        assert_eq!(
            owner.await.unwrap(),
            Ok(Outcome {
                value: 1,
                revision: 1
            })
        );
        for w in waiters {
            assert_eq!(
                w.await.unwrap(),
                Ok(Outcome {
                    value: 1,
                    revision: 1
                })
            );
        }
        assert_eq!(counts(&r.shared.path), (1, 1, 1));
        assert_eq!(
            r.shared.stats.attempts.load(Ordering::Relaxed),
            if mode.flight() { 1 } else { 8 }
        );
    }
}
#[tokio::test]
async fn mismatch_rejected_in_flight_and_cached() {
    for mode in [Mode::Flight, Mode::Moka, Mode::Quick] {
        let (_d, r) = setup(mode, 8, 8);
        let g = gate(&r);
        let owner = spawn(&r, key(1), input(10));
        take(&g.entered, 1).await;
        assert_eq!(r.call(key(1), input(11)).await, Err(Error::Conflict));
        g.release.add_permits(1);
        assert!(owner.await.unwrap().is_ok());
        *r.shared.gate.lock().unwrap() = None;
        assert_eq!(r.call(key(1), input(11)).await, Err(Error::Conflict));
        assert_eq!(counts(&r.shared.path), (1, 1, 1));
    }
}
#[tokio::test]
async fn cancelled_owner_does_not_cancel_work_or_waiters() {
    for mode in [Mode::Flight, Mode::Moka, Mode::Quick] {
        let (_d, r) = setup(mode, 1, 8);
        let g = gate(&r);
        let owner = spawn(&r, key(1), input(10));
        take(&g.entered, 1).await;
        let waiter = spawn(&r, key(1), input(10));
        take(&g.joined, 1).await;
        owner.abort();
        assert!(owner.await.unwrap_err().is_cancelled());
        g.release.add_permits(1);
        assert!(waiter.await.unwrap().is_ok());
        assert_eq!(counts(&r.shared.path), (1, 1, 1));
    }
}
#[tokio::test]
async fn expiration_and_capacity_eviction_fall_back_to_durable_receipt() {
    for mode in [Mode::Moka, Mode::Quick] {
        let (_d, r) = setup(mode, 8, 1);
        let result = r.call(key(0), input(10)).await;
        assert!(result.is_ok());
        r.shared.offset.store(60_001, Ordering::Relaxed);
        assert_eq!(r.call(key(0), input(10)).await, result);
        assert_eq!(r.shared.stats.attempts.load(Ordering::Relaxed), 2);
        for id in 1..100 {
            assert!(r.call(key(id), input(10)).await.is_ok());
        }
        if let Cache::Moka(c) = &r.shared.cache {
            c.run_pending_tasks();
        }
        // Find an actually evicted key; admission policies need not evict the oldest.
        let evicted = (0..100)
            .find(|id| r.shared.cache.get(&key(id), r.shared.now()).is_none())
            .expect("bounded cache must evict under pressure");
        let attempts = r.shared.stats.attempts.load(Ordering::Relaxed);
        assert!(r.call(key(evicted), input(10)).await.is_ok());
        assert_eq!(
            r.shared.stats.attempts.load(Ordering::Relaxed),
            attempts + 1
        );
        assert_eq!(counts(&r.shared.path), (100, 100, 100));
    }
}
#[tokio::test]
async fn runtime_restart_replays_and_independent_instances_arbitrate() {
    for mode in Mode::ALL {
        let (d, r) = setup(mode, 8, 8);
        let path = r.shared.path.clone();
        let expected = r.call(key(1), input(10)).await;
        drop(r);
        let a = Runtime::new(&path, mode, 8, 8, 60_000);
        assert_eq!(a.call(key(1), input(10)).await, expected);
        let b = Runtime::new(&path, mode, 8, 8, 60_000);
        let ga = gate(&a);
        let gb = gate(&b);
        let ja = spawn(&a, key(2), input(10));
        let jb = spawn(&b, key(2), input(10));
        take(&ga.entered, 1).await;
        take(&gb.entered, 1).await;
        ga.release.add_permits(1);
        gb.release.add_permits(1);
        assert_eq!(ja.await.unwrap(), jb.await.unwrap());
        assert_eq!(counts(&path), (2, 2, 2));
        drop(d);
    }
}
#[tokio::test]
async fn authorization_is_checked_on_cached_and_inflight_delivery() {
    for mode in [Mode::Moka, Mode::Quick] {
        let (_d, r) = setup(mode, 8, 8);
        assert!(r.call(key(1), input(10)).await.is_ok());
        r.shared.denied.lock().unwrap().insert("alice".into());
        assert_eq!(r.call(key(1), input(10)).await, Err(Error::Denied));
        r.shared.denied.lock().unwrap().clear();
        let g = gate(&r);
        let j = spawn(&r, key(2), input(10));
        take(&g.entered, 1).await;
        r.shared.denied.lock().unwrap().insert("alice".into());
        g.release.add_permits(1);
        assert_eq!(j.await.unwrap(), Err(Error::Denied));
        assert_eq!(counts(&r.shared.path), (2, 2, 2));
    }
}
#[tokio::test]
async fn real_commit_unknown_is_not_cached_as_success_and_retry_resolves() {
    for mode in Mode::ALL {
        let (_d, r) = setup(mode, 8, 8);
        r.shared.unknown.store(true, Ordering::SeqCst);
        assert_eq!(r.call(key(1), input(10)).await, Err(Error::Unknown));
        assert_eq!(counts(&r.shared.path), (1, 1, 1));
        assert!(r.shared.cache.get(&key(1), r.shared.now()).is_none());
        assert_eq!(
            r.call(key(1), input(10)).await,
            Ok(Outcome {
                value: 1,
                revision: 1
            })
        );
        assert_eq!(r.shared.stats.attempts.load(Ordering::Relaxed), 2);
        assert_eq!(counts(&r.shared.path), (1, 1, 1));
    }
}
#[tokio::test]
async fn different_identity_or_scope_never_collapses_equal_payloads() {
    for mode in Mode::ALL {
        let (_d, r) = setup(mode, 8, 8);
        let mut keys = vec![key(1), key(2), key(1), key(1), key(1)];
        keys[2].tenant = "other".into();
        keys[3].principal = "bob".into();
        keys[4].operation = "increment/v2".into();
        for k in keys {
            assert!(r.call(k, input(10)).await.is_ok());
        }
        assert_eq!(counts(&r.shared.path), (5, 5, 5));
    }
}
#[tokio::test]
async fn admission_is_bounded_and_live_jobs_survive_cache_expiration() {
    for mode in Mode::ALL {
        let (_d, r) = setup(mode, 1, 1);
        let g = gate(&r);
        let j = spawn(&r, key(1), input(10));
        take(&g.entered, 1).await;
        r.shared.offset.store(1_000_000, Ordering::Relaxed);
        assert_eq!(r.call(key(2), input(10)).await, Err(Error::Busy));
        let waiter = if mode.flight() {
            let w = spawn(&r, key(1), input(10));
            take(&g.joined, 1).await;
            Some(w)
        } else {
            None
        };
        g.release.add_permits(1);
        assert!(j.await.unwrap().is_ok());
        if let Some(w) = waiter {
            assert!(w.await.unwrap().is_ok());
        }
        assert_eq!(counts(&r.shared.path), (1, 1, 1));
    }
}

#[tokio::test]
async fn worker_panic_resolves_waiters_and_releases_flight_for_retry() {
    for mode in Mode::ALL {
        let (_d, r) = setup(mode, 8, 8);
        r.shared.panic_worker.store(true, Ordering::SeqCst);
        let g = gate(&r);
        let owner = spawn(&r, key(1), input(10));
        take(&g.entered, 1).await;
        let waiters: Vec<_> = if mode.flight() {
            (0..3).map(|_| spawn(&r, key(1), input(10))).collect()
        } else {
            Vec::new()
        };
        if mode.flight() {
            take(&g.joined, 3).await;
        }
        g.release.add_permits(1);
        assert_eq!(owner.await.unwrap(), Err(Error::Unknown));
        for w in waiters {
            assert_eq!(w.await.unwrap(), Err(Error::Unknown));
        }
        *r.shared.gate.lock().unwrap() = None;
        assert_eq!(counts(&r.shared.path), (0, 0, 0));
        assert!(r.shared.flights.lock().unwrap().is_empty());
        assert!(r.call(key(1), input(10)).await.is_ok());
        assert_eq!(counts(&r.shared.path), (1, 1, 1));
    }
}
#[tokio::test]
async fn payload_and_joining_callers_are_bounded() {
    let (_d, r) = setup(Mode::Flight, 1, 8);
    assert_eq!(
        r.call(key(1), input(1024 * 1024 + 1)).await,
        Err(Error::TooLarge)
    );
    let g = gate(&r);
    let owner = spawn(&r, key(1), input(1));
    take(&g.entered, 1).await;
    let waiters: Vec<_> = (0..31).map(|_| spawn(&r, key(1), input(1))).collect();
    take(&g.joined, 31).await;
    assert_eq!(r.call(key(1), input(1)).await, Err(Error::Busy));
    g.release.add_permits(1);
    assert!(owner.await.unwrap().is_ok());
    for w in waiters {
        assert!(w.await.unwrap().is_ok());
    }
    assert_eq!(counts(&r.shared.path), (1, 1, 1));
}

#[tokio::test]
async fn cancelled_follower_leaves_owner_and_other_waiter_alive() {
    for mode in [Mode::Flight, Mode::Moka, Mode::Quick] {
        let (_d, r) = setup(mode, 1, 8);
        let g = gate(&r);
        let owner = spawn(&r, key(1), input(10));
        take(&g.entered, 1).await;
        let cancelled = spawn(&r, key(1), input(10));
        let survivor = spawn(&r, key(1), input(10));
        take(&g.joined, 2).await;
        cancelled.abort();
        assert!(cancelled.await.unwrap_err().is_cancelled());
        g.release.add_permits(1);
        assert!(owner.await.unwrap().is_ok());
        assert!(survivor.await.unwrap().is_ok());
        assert_eq!(counts(&r.shared.path), (1, 1, 1));
    }
}

#[tokio::test]
async fn baseline_inflight_mismatched_inputs_have_one_durable_winner() {
    let (_d, r) = setup(Mode::Baseline, 8, 8);
    let g = gate(&r);
    let a = spawn(&r, key(1), input(10));
    let b = spawn(&r, key(1), input(11));
    take(&g.entered, 2).await;
    g.release.add_permits(2);
    let a = a.await.unwrap();
    let b = b.await.unwrap();
    assert!(matches!(
        (&a, &b),
        (Ok(_), Err(Error::Conflict)) | (Err(Error::Conflict), Ok(_))
    ));
    assert_eq!(counts(&r.shared.path), (1, 1, 1));
}
