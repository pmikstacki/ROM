//! Throwaway pure-read experiment. Snapshot is the authority; Salsa is a projection.
use salsa::Setter;
use std::collections::BTreeMap;
use std::sync::{
    Arc,
    atomic::{AtomicBool, AtomicUsize, Ordering},
};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Task {
    pub id: u64,
    pub owner: u64,
    pub done: bool,
    pub title: String,
}

/// A single committed resource/policy snapshot from the authoritative store.
/// The prototype supplies it in memory, and does not implement a database.
#[derive(Clone, Debug)]
pub struct Snapshot {
    pub revision: u64,
    pub tasks: BTreeMap<u64, Task>,
    pub actor: u64,
    pub enabled: bool,
}

impl Snapshot {
    pub fn fixture() -> Self {
        Self {
            revision: 1,
            tasks: [
                Task {
                    id: 1,
                    owner: 7,
                    done: false,
                    title: "one".into(),
                },
                Task {
                    id: 2,
                    owner: 7,
                    done: true,
                    title: "two".into(),
                },
                Task {
                    id: 3,
                    owner: 8,
                    done: false,
                    title: "three".into(),
                },
            ]
            .into_iter()
            .map(|t| (t.id, t))
            .collect(),
            actor: 7,
            enabled: true,
        }
    }
}

pub fn plain_open_ids(snapshot: &Snapshot) -> Vec<u64> {
    snapshot
        .tasks
        .values()
        .filter(|t| snapshot.enabled && t.owner == snapshot.actor && !t.done)
        .map(|t| t.id)
        .collect()
}

#[salsa::input]
struct TaskFact {
    #[returns(copy)]
    id: u64,
    #[returns(copy)]
    owner: u64,
    #[returns(copy)]
    done: bool,
    title: String,
}

#[salsa::input]
struct Catalog {
    tasks: Vec<TaskFact>,
}

#[salsa::input]
struct ActorPolicy {
    #[returns(copy)]
    actor: u64,
    #[returns(copy)]
    enabled: bool,
}

#[derive(Default)]
struct Counters {
    membership: AtomicUsize,
    list: AtomicUsize,
    count: AtomicUsize,
}

#[salsa::db]
trait Db: salsa::Database {
    fn counters(&self) -> &Counters;
    fn external_enabled(&self) -> bool;
}

#[salsa::db]
#[derive(Default, Clone)]
struct ReadDb {
    storage: salsa::Storage<Self>,
    counters: Arc<Counters>,
    // Deliberately outside Salsa: used ONLY by the incorrect negative control.
    external_enabled: Arc<AtomicBool>,
}
#[salsa::db]
impl salsa::Database for ReadDb {}
#[salsa::db]
impl Db for ReadDb {
    fn counters(&self) -> &Counters {
        &self.counters
    }
    fn external_enabled(&self) -> bool {
        self.external_enabled.load(Ordering::Relaxed)
    }
}

// These counters are observational instrumentation only. They never affect results,
// authorize requests, mutate resources, send events, or perform durable effects.
#[salsa::tracked(returns(copy))]
fn visible_open(db: &dyn Db, task: TaskFact, policy: ActorPolicy) -> bool {
    db.counters().membership.fetch_add(1, Ordering::Relaxed);
    policy.enabled(db) && task.owner(db) == policy.actor(db) && !task.done(db)
}

#[salsa::tracked]
fn open_ids(db: &dyn Db, catalog: Catalog, policy: ActorPolicy) -> Vec<u64> {
    db.counters().list.fetch_add(1, Ordering::Relaxed);
    catalog
        .tasks(db)
        .iter()
        .copied()
        .filter(|task| visible_open(db, *task, policy))
        .map(|task| task.id(db))
        .collect()
}

#[salsa::tracked(returns(copy))]
fn open_count(db: &dyn Db, catalog: Catalog, policy: ActorPolicy) -> usize {
    db.counters().count.fetch_add(1, Ordering::Relaxed);
    open_ids(db, catalog, policy).len()
}

/// Deliberately WRONG: external permission change cannot invalidate this memo.
#[salsa::tracked]
fn wrong_open_ids(db: &dyn Db, catalog: Catalog, policy: ActorPolicy) -> Vec<u64> {
    if db.external_enabled() {
        open_ids(db, catalog, policy).clone()
    } else {
        vec![]
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Executions {
    pub membership: usize,
    pub list: usize,
    pub count: usize,
}

/// Internal adapter, not a proposed user-facing Resource API.
/// A real registry would register this read against the existing Task resource.
/// One model is scoped to one authoritative dataset and one actor context.
pub struct RegisteredTaskReads {
    db: ReadDb,
    facts: BTreeMap<u64, TaskFact>,
    catalog: Catalog,
    policy: ActorPolicy,
    applied_revision: Option<u64>,
}

impl Default for RegisteredTaskReads {
    fn default() -> Self {
        let db = ReadDb::default();
        let catalog = Catalog::new(&db, vec![]);
        let policy = ActorPolicy::new(&db, 0, false);
        Self {
            db,
            facts: BTreeMap::new(),
            catalog,
            policy,
            applied_revision: None,
        }
    }
}

impl RegisteredTaskReads {
    /// Exclusive publication boundary: no read can see partly applied fields.
    /// Full snapshot reconciliation is O(n); changefeed optimization is not implemented.
    pub fn apply(&mut self, snapshot: &Snapshot) -> Result<usize, &'static str> {
        if self
            .applied_revision
            .is_some_and(|r| snapshot.revision <= r)
        {
            return Err("snapshot revision must strictly increase");
        }
        if snapshot.tasks.iter().any(|(id, task)| *id != task.id) {
            return Err("task key/id mismatch");
        }
        let mut setters = 0;
        for (id, task) in &snapshot.tasks {
            if let Some(fact) = self.facts.get(id).copied() {
                if fact.owner(&self.db) != task.owner {
                    fact.set_owner(&mut self.db).to(task.owner);
                    setters += 1;
                }
                if fact.done(&self.db) != task.done {
                    fact.set_done(&mut self.db).to(task.done);
                    setters += 1;
                }
                if fact.title(&self.db) != &task.title {
                    fact.set_title(&mut self.db).to(task.title.clone());
                    setters += 1;
                }
            } else {
                self.facts.insert(
                    *id,
                    TaskFact::new(&self.db, task.id, task.owner, task.done, task.title.clone()),
                );
            }
        }
        // Removing handles does not reclaim Salsa input storage; see ASSESSMENT.
        self.facts.retain(|id, _| snapshot.tasks.contains_key(id));
        let tasks: Vec<_> = self.facts.values().copied().collect();
        if self.catalog.tasks(&self.db) != &tasks {
            self.catalog.set_tasks(&mut self.db).to(tasks);
            setters += 1;
        }
        if self.policy.actor(&self.db) != snapshot.actor {
            self.policy.set_actor(&mut self.db).to(snapshot.actor);
            setters += 1;
        }
        if self.policy.enabled(&self.db) != snapshot.enabled {
            self.policy.set_enabled(&mut self.db).to(snapshot.enabled);
            setters += 1;
        }
        self.applied_revision = Some(snapshot.revision);
        Ok(setters)
    }

    pub fn open_ids(&self) -> Vec<u64> {
        open_ids(&self.db, self.catalog, self.policy).clone()
    }
    pub fn open_count(&self) -> usize {
        open_count(&self.db, self.catalog, self.policy)
    }
    pub fn executions(&self) -> Executions {
        let c = &self.db.counters;
        Executions {
            membership: c.membership.load(Ordering::Relaxed),
            list: c.list.load(Ordering::Relaxed),
            count: c.count.load(Ordering::Relaxed),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn warm() -> (Snapshot, RegisteredTaskReads) {
        let s = Snapshot::fixture();
        let mut model = RegisteredTaskReads::default();
        model.apply(&s).unwrap();
        assert_eq!(model.open_ids(), plain_open_ids(&s));
        assert_eq!(model.open_count(), 1);
        (s, model)
    }

    #[test]
    fn unchanged_reads_reuse_memos() {
        let (_, model) = warm();
        let before = model.executions();
        for _ in 0..1_000 {
            assert_eq!(model.open_ids(), vec![1]);
            assert_eq!(model.open_count(), 1);
        }
        assert_eq!(model.executions(), before);
    }

    #[test]
    fn title_edit_is_precise_because_fields_are_separate() {
        let (mut s, mut model) = warm();
        let before = model.executions();
        s.revision += 1;
        s.tasks.get_mut(&1).unwrap().title = "renamed".into();
        assert_eq!(model.apply(&s), Ok(1));
        assert_eq!(model.open_ids(), plain_open_ids(&s));
        assert_eq!(model.open_count(), 1);
        assert_eq!(model.executions(), before);
    }

    #[test]
    fn completion_edit_reexecutes_one_membership_and_derived_reads() {
        let (mut s, mut model) = warm();
        let before = model.executions();
        s.revision += 1;
        s.tasks.get_mut(&2).unwrap().done = false;
        assert_eq!(model.apply(&s), Ok(1));
        assert_eq!(model.open_ids(), vec![1, 2]);
        assert_eq!(model.open_count(), 2);
        assert_eq!(model.open_ids(), plain_open_ids(&s));
        assert_eq!(
            model.executions(),
            Executions {
                membership: before.membership + 1,
                list: before.list + 1,
                count: before.count + 1
            }
        );
    }

    #[test]
    fn policy_revocation_and_actor_switch_invalidate() {
        let (mut s, mut model) = warm();
        s.revision += 1;
        s.enabled = false;
        model.apply(&s).unwrap();
        assert_eq!(model.open_ids(), plain_open_ids(&s));
        assert_eq!(model.open_count(), 0);
        s.revision += 1;
        s.enabled = true;
        s.actor = 8;
        model.apply(&s).unwrap();
        assert_eq!(model.open_ids(), vec![3]);
        assert_eq!(model.open_ids(), plain_open_ids(&s));
    }

    #[test]
    fn insertion_deletion_and_reinsert_update_catalog() {
        let (mut s, mut model) = warm();
        s.revision += 1;
        s.tasks.remove(&1);
        model.apply(&s).unwrap();
        assert_eq!(model.open_ids(), plain_open_ids(&s));
        assert_eq!(model.open_count(), 0);
        s.revision += 1;
        s.tasks.insert(
            1,
            Task {
                id: 1,
                owner: 7,
                done: false,
                title: "replacement".into(),
            },
        );
        model.apply(&s).unwrap();
        assert_eq!(model.open_ids(), vec![1]);
        s.revision += 1;
        s.tasks.insert(
            4,
            Task {
                id: 4,
                owner: 7,
                done: false,
                title: "new".into(),
            },
        );
        model.apply(&s).unwrap();
        assert_eq!(model.open_ids(), plain_open_ids(&s));
        assert_eq!(model.open_count(), 2);
    }

    #[test]
    fn untracked_external_policy_negative_control_returns_stale_authorization() {
        let (_, model) = warm();
        model.db.external_enabled.store(true, Ordering::Relaxed);
        assert_eq!(
            wrong_open_ids(&model.db, model.catalog, model.policy),
            &vec![1]
        );
        model.db.external_enabled.store(false, Ordering::Relaxed);
        // Intended negative-control failure: cached data remains visible after revoke.
        assert!(!model.db.external_enabled());
        assert_eq!(
            wrong_open_ids(&model.db, model.catalog, model.policy),
            &vec![1]
        );
    }

    #[test]
    fn revision_replay_rejected_and_noop_snapshot_preserves_memos() {
        let (mut s, mut model) = warm();
        let before = model.executions();
        assert!(model.apply(&s).is_err());
        s.revision += 1;
        assert_eq!(model.apply(&s), Ok(0));
        assert_eq!(model.open_ids(), plain_open_ids(&s));
        assert_eq!(model.open_count(), 1);
        assert_eq!(model.executions(), before);
    }

    #[test]
    fn dependency_change_with_equal_boolean_backdates_derived_reads() {
        let (mut s, mut model) = warm();
        let before = model.executions();
        s.revision += 1;
        s.tasks.get_mut(&3).unwrap().owner = 9;
        model.apply(&s).unwrap();
        assert_eq!(model.open_ids(), plain_open_ids(&s));
        assert_eq!(model.open_count(), 1);
        assert_eq!(
            model.executions(),
            Executions {
                membership: before.membership + 1,
                ..before
            }
        );
    }
}
