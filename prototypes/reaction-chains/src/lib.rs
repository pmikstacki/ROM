//! Deterministic after-commit simulation. All database operations and durability are modeled.
use std::collections::{BTreeMap, BTreeSet, VecDeque};

pub const VALUE: u8 = 1;
pub const NOTE: u8 = 2;
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Resource {
    pub value: i64,
    pub note: i64,
    pub revision: u64,
}
#[derive(Clone, Debug)]
pub struct Event {
    pub id: u64,
    pub root: u64,
    pub resource: usize,
    pub value: i64,
    pub changed: u8,
    pub revision: u64,
    pub depth: usize,
    pub path: Vec<usize>,
}
#[derive(Clone, Copy, Debug)]
pub enum Transform {
    Copy,
    Invert,
    StepTo(i64),
    AddOne,
}
#[derive(Clone, Debug)]
pub struct Rule {
    pub source: usize,
    pub target: usize,
    pub depends: u8,
    pub transform: Transform,
    pub fail_first: usize,
    pub permanent: bool,
}
pub fn rule(source: usize, target: usize, transform: Transform) -> Rule {
    Rule {
        source,
        target,
        depends: VALUE,
        transform,
        fail_first: 0,
        permanent: false,
    }
}
#[derive(Clone, Debug)]
pub struct Policy {
    pub dedup: bool,
    pub noop: bool,
    pub filters: bool,
    pub visited: bool,
    pub max_hops: usize,
    pub max_work: usize,
    pub queue_cap: usize,
    pub max_attempts: usize,
}
impl Policy {
    pub fn naive() -> Self {
        Self {
            dedup: false,
            noop: false,
            filters: false,
            visited: false,
            max_hops: usize::MAX,
            max_work: usize::MAX,
            queue_cap: usize::MAX,
            max_attempts: 3,
        }
    }
    pub fn layered() -> Self {
        Self {
            dedup: true,
            noop: true,
            filters: true,
            visited: false,
            max_hops: 16,
            max_work: 64,
            queue_cap: 4,
            max_attempts: 3,
        }
    }
}
#[derive(Clone, Debug, Default)]
pub struct Counts {
    pub attempts: usize,
    pub writes: usize,
    pub events: usize,
    pub receipt_reads: usize,
    pub resource_reads: usize,
    pub transactions: usize,
    pub redundant_roundtrips: usize,
    pub noops: usize,
    pub duplicates: usize,
    pub filtered: usize,
    pub budget_stops: usize,
    pub visited_stops: usize,
    pub failures: usize,
    pub dead_letters: usize,
    pub ready_peak: usize,
    pub backlog_peak: usize,
    pub ticks: u64,
    pub watchdog: bool,
}
impl Counts {
    pub fn roundtrips(&self) -> usize {
        self.receipt_reads + self.resource_reads + self.transactions
    }
}
#[derive(Clone, Debug)]
struct Job {
    event: Event,
    rule: usize,
    attempts: usize,
    due: u64,
}
impl Job {
    fn identity(&self) -> (u64, usize) {
        (self.event.id, self.rule)
    }
}
/// Stand-in for durable storage. Cloning this value models a restart checkpoint;
/// it is NOT serialization, filesystem I/O, process death, or a transaction test.
#[derive(Clone)]
struct Store {
    resources: Vec<Resource>,
    journal: Vec<Event>,
    receipts: BTreeSet<(u64, usize)>,
    backlog: VecDeque<Job>,
    root_work: BTreeMap<u64, usize>,
    failed: Vec<Job>,
    counts: Counts,
}
pub struct Engine {
    store: Store,
    ready: VecDeque<Job>,
    rules: Vec<Rule>,
    policy: Policy,
    crash_once: bool,
    pub restarts: usize,
}
impl Engine {
    pub fn new(resources: usize, rules: Vec<Rule>, policy: Policy) -> Self {
        assert!(policy.queue_cap > 0 && policy.max_attempts > 0);
        Self {
            store: Store {
                resources: vec![Resource::default(); resources],
                journal: vec![],
                receipts: BTreeSet::new(),
                backlog: VecDeque::new(),
                root_work: BTreeMap::new(),
                failed: vec![],
                counts: Counts::default(),
            },
            ready: VecDeque::new(),
            rules,
            policy,
            crash_once: false,
            restarts: 0,
        }
    }
    pub fn counts(&self) -> &Counts {
        &self.store.counts
    }
    pub fn resource(&self, id: usize) -> Resource {
        self.store.resources[id]
    }
    pub fn pending(&self) -> usize {
        self.ready.len() + self.store.backlog.len()
    }
    pub fn seed(&mut self, id: usize, value: i64, note: i64) {
        self.store.counts.attempts += 1;
        self.store.counts.resource_reads += 1;
        let old = self.store.resources[id];
        let changed = (u8::from(old.value != value) * VALUE) | (u8::from(old.note != note) * NOTE);
        self.store.resources[id] = Resource {
            value,
            note,
            revision: old.revision + 1,
        };
        let event_id = self.store.journal.len() as u64 + 1;
        self.commit_event(Event {
            id: event_id,
            root: event_id,
            resource: id,
            value,
            changed,
            revision: old.revision + 1,
            depth: 0,
            path: vec![id],
        });
    }
    fn commit_event(&mut self, event: Event) {
        self.store.counts.writes += 1;
        self.store.counts.transactions += 1;
        self.store.counts.events += 1;
        self.store.journal.push(event.clone());
        for (id, rule) in self.rules.iter().enumerate() {
            if rule.source != event.resource {
                continue;
            }
            if self.policy.filters && event.changed & rule.depends == 0 {
                self.store.counts.filtered += 1;
                continue;
            }
            self.store.backlog.push_back(Job {
                event: event.clone(),
                rule: id,
                attempts: 0,
                due: self.store.counts.ticks,
            });
        }
        self.store.counts.backlog_peak =
            self.store.counts.backlog_peak.max(self.store.backlog.len());
    }
    pub fn duplicate_first_delivery(&mut self) {
        let job = self.store.backlog.front().unwrap().clone();
        self.store.backlog.push_back(job);
        self.store.counts.backlog_peak =
            self.store.counts.backlog_peak.max(self.store.backlog.len());
    }
    pub fn crash_after_next_commit(&mut self) {
        self.crash_once = true;
    }
    fn fill(&mut self) {
        let count = self.store.backlog.len();
        for _ in 0..count {
            let job = self.store.backlog.pop_front().unwrap();
            if self.ready.len() < self.policy.queue_cap && job.due <= self.store.counts.ticks {
                self.ready.push_back(job);
            } else {
                self.store.backlog.push_back(job);
            }
        }
        self.store.counts.ready_peak = self.store.counts.ready_peak.max(self.ready.len());
    }
    fn failed(&mut self, mut job: Job) {
        self.store.counts.failures += 1;
        if self.rules[job.rule].permanent || job.attempts >= self.policy.max_attempts {
            self.store.counts.dead_letters += 1;
            self.store.failed.push(job);
        } else {
            job.due = self.store.counts.ticks + (1_u64 << (job.attempts - 1).min(4));
            self.store.backlog.push_back(job);
            self.store.counts.backlog_peak =
                self.store.counts.backlog_peak.max(self.store.backlog.len());
        }
    }
    /// Process at most `limit` deliveries. Watchdog is an experimental guard, not
    /// a successful quiescence result and not a production loop policy.
    pub fn run(&mut self, limit: usize) {
        for _ in 0..limit {
            self.fill();
            if self.ready.is_empty() {
                if let Some(next) = self.store.backlog.iter().map(|j| j.due).min() {
                    self.store.counts.ticks = next;
                    self.fill();
                } else {
                    return;
                }
            }
            let mut job = self.ready.pop_front().unwrap();
            let rule = self.rules[job.rule].clone();
            if self.policy.visited && job.event.path.contains(&rule.target) {
                self.store.counts.visited_stops += 1;
                continue;
            }
            if self.policy.dedup {
                self.store.counts.receipt_reads += 1;
                if self.store.receipts.contains(&job.identity()) {
                    self.store.counts.duplicates += 1;
                    self.store.counts.redundant_roundtrips += 1;
                    continue;
                }
            }
            let work = self.store.root_work.entry(job.event.root).or_default();
            if job.event.depth + 1 > self.policy.max_hops || *work >= self.policy.max_work {
                self.store.counts.budget_stops += 1;
                self.store.failed.push(job);
                continue;
            }
            *work += 1;
            job.attempts += 1;
            self.store.counts.attempts += 1;
            self.store.counts.resource_reads += 1;
            if rule.permanent || job.attempts <= rule.fail_first {
                self.failed(job);
                continue;
            }
            let old = self.store.resources[rule.target];
            let value = match rule.transform {
                Transform::Copy => job.event.value,
                Transform::Invert => 1 - job.event.value,
                Transform::StepTo(max) => (job.event.value + 1).min(max),
                Transform::AddOne => old.value + 1,
            };
            if old.value == value {
                self.store.counts.redundant_roundtrips += 1 + usize::from(self.policy.dedup);
                if self.policy.noop {
                    self.store.counts.noops += 1;
                    if self.policy.dedup {
                        self.store.receipts.insert(job.identity());
                        self.store.counts.transactions += 1;
                        self.store.counts.redundant_roundtrips += 1;
                    }
                    continue;
                }
                self.store.counts.redundant_roundtrips += 1;
            }
            // State + event + action identity are one indivisible MODEL transition.
            self.store.resources[rule.target] = Resource {
                value,
                revision: old.revision + 1,
                ..old
            };
            if self.policy.dedup {
                self.store.receipts.insert(job.identity());
            }
            let mut path = job.event.path.clone();
            path.push(rule.target);
            let event = Event {
                id: self.store.journal.len() as u64 + 1,
                root: job.event.root,
                resource: rule.target,
                value,
                changed: if old.value == value { 0 } else { VALUE },
                revision: old.revision + 1,
                depth: job.event.depth + 1,
                path,
            };
            self.commit_event(event);
            if self.crash_once {
                // Commit persisted, source delivery acknowledgement did not.
                // Recover all unacknowledged work; retain receipts and budgets.
                self.crash_once = false;
                self.store.backlog.push_front(job);
                self.store.backlog.extend(self.ready.drain(..));
                self.store.counts.backlog_peak =
                    self.store.counts.backlog_peak.max(self.store.backlog.len());
                self.store = self.store.clone();
                self.restarts += 1;
            }
        }
        self.store.counts.watchdog = self.pending() > 0;
    }
}

pub fn policies() -> Vec<(&'static str, Policy)> {
    let naive = Policy::naive();
    vec![
        ("naive", naive.clone()),
        (
            "dedup_only",
            Policy {
                dedup: true,
                ..naive.clone()
            },
        ),
        (
            "noop_only",
            Policy {
                noop: true,
                ..naive.clone()
            },
        ),
        (
            "filters_only",
            Policy {
                filters: true,
                ..naive.clone()
            },
        ),
        (
            "hop_only",
            Policy {
                max_hops: 8,
                ..naive.clone()
            },
        ),
        (
            "work_only",
            Policy {
                max_work: 12,
                ..naive.clone()
            },
        ),
        (
            "visited_only",
            Policy {
                visited: true,
                ..naive
            },
        ),
        (
            "queue_only",
            Policy {
                queue_cap: 4,
                ..Policy::naive()
            },
        ),
        (
            "semantic_without_budgets",
            Policy {
                max_hops: usize::MAX,
                max_work: usize::MAX,
                ..Policy::layered()
            },
        ),
        ("layered", Policy::layered()),
    ]
}
pub fn scenario(name: &str, policy: Policy) -> Engine {
    let (n, rules) = match name {
        "acyclic" => (
            3,
            vec![rule(0, 1, Transform::Copy), rule(1, 2, Transform::Copy)],
        ),
        "echo" | "unrelated" => (
            2,
            vec![rule(0, 1, Transform::Copy), rule(1, 0, Transform::Copy)],
        ),
        "oscillation" => (
            2,
            vec![rule(0, 1, Transform::Copy), rule(1, 0, Transform::Invert)],
        ),
        "converging" => (
            2,
            vec![
                rule(0, 1, Transform::StepTo(4)),
                rule(1, 0, Transform::StepTo(4)),
            ],
        ),
        "duplicate" | "restart" => (2, vec![rule(0, 1, Transform::AddOne)]),
        "failure" | "transient" | "permanent" => {
            let mut r = rule(0, 1, Transform::Copy);
            r.fail_first = if name == "transient" { 2 } else { usize::MAX };
            r.permanent = name == "permanent";
            (2, vec![r])
        }
        "fanout" => (
            65,
            (1..65)
                .map(|target| rule(0, target, Transform::Copy))
                .collect(),
        ),
        _ => panic!("unknown scenario"),
    };
    let mut engine = Engine::new(n, rules, policy);
    if name == "unrelated" {
        engine.seed(0, 0, 1);
    } else {
        engine.seed(0, 1, 0);
    }
    if name == "duplicate" {
        engine.duplicate_first_delivery();
    }
    if name == "restart" {
        engine.crash_after_next_commit();
    }
    engine.run(128);
    engine
}

/// Latest-state projection versus domain events: intentionally separate APIs.
pub fn latest_projection(
    events: &[(u64, i64)],
    version_guard: bool,
    coalesce: bool,
) -> (i64, usize) {
    let input: Vec<_> = if coalesce {
        events
            .iter()
            .max_by_key(|(rev, _)| rev)
            .copied()
            .into_iter()
            .collect()
    } else {
        events.to_vec()
    };
    let (mut revision, mut value, mut writes) = (0, 0, 0);
    for (rev, next) in input {
        if version_guard && rev <= revision {
            continue;
        }
        revision = rev;
        value = next;
        writes += 1;
    }
    (value, writes)
}
pub fn domain_total(events: &[(u64, i64)], unsafe_coalesce: bool) -> i64 {
    let input: Vec<_> = if unsafe_coalesce {
        events.last().copied().into_iter().collect()
    } else {
        events.to_vec()
    };
    let mut seen = BTreeSet::new();
    input
        .into_iter()
        .filter(|(id, _)| seen.insert(*id))
        .map(|(_, delta)| delta)
        .sum()
}
/// A finite batch models buffering/recovery for one ordered resource history.
/// It must receive a contiguous sequence from revision one; a production cursor
/// would supply the start and enforce a bounded gap-wait/recovery policy.
pub fn ordered_history(events: &[(u64, i64)]) -> Result<Vec<i64>, &'static str> {
    let mut unique = BTreeMap::new();
    for &(revision, value) in events {
        if unique
            .insert(revision, value)
            .is_some_and(|old| old != value)
        {
            return Err("identity_conflict");
        }
    }
    let mut output = Vec::new();
    for (index, (revision, value)) in unique.into_iter().enumerate() {
        if revision != index as u64 + 1 {
            return Err("gap");
        }
        output.push(value);
    }
    Ok(output)
}
/// Special case: inflationary set union on a finite graph, no external effects.
pub fn monotone_closure(
    nodes: usize,
    edges: &[(usize, usize)],
    seed: usize,
) -> (BTreeSet<usize>, usize) {
    let mut reached = BTreeSet::from([seed]);
    let mut frontier = VecDeque::from([seed]);
    let mut steps = 0;
    while let Some(source) = frontier.pop_front() {
        steps += 1;
        for &(a, b) in edges {
            assert!(a < nodes && b < nodes);
            if a == source && reached.insert(b) {
                frontier.push_back(b);
            }
        }
    }
    (reached, steps)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn acyclic_all_policies_agree() {
        for (_, p) in policies() {
            let e = scenario("acyclic", p);
            assert_eq!(e.resource(2).value, 1);
            assert_eq!(e.counts().writes, 3);
            assert!(!e.counts().watchdog);
        }
    }
    #[test]
    fn stable_delivery_ids_do_not_stop_new_identity_echo() {
        for p in [
            Policy::naive(),
            Policy {
                dedup: true,
                ..Policy::naive()
            },
        ] {
            let e = scenario("echo", p);
            assert!(e.counts().watchdog);
            assert_eq!(e.counts().writes, 129);
            assert_eq!(e.counts().duplicates, 0);
        }
    }
    #[test]
    fn noop_stops_echo_but_not_oscillation() {
        let p = Policy {
            noop: true,
            ..Policy::naive()
        };
        let echo = scenario("echo", p.clone());
        assert_eq!(echo.counts().writes, 2);
        assert_eq!(echo.counts().noops, 1);
        assert!(!echo.counts().watchdog);
        assert!(scenario("oscillation", p).counts().watchdog);
    }
    #[test]
    fn filters_skip_unrelated_changes_but_cannot_stop_real_oscillation() {
        let p = Policy {
            filters: true,
            ..Policy::naive()
        };
        let unrelated = scenario("unrelated", p.clone());
        assert_eq!(unrelated.counts().attempts, 1);
        assert_eq!(unrelated.counts().filtered, 1);
        assert!(scenario("oscillation", p).counts().watchdog);
    }
    #[test]
    fn filter_false_negative_loses_valid_work() {
        let mut wrong = rule(0, 1, Transform::Copy);
        wrong.depends = NOTE;
        let mut e = Engine::new(2, vec![wrong], Policy::layered());
        e.seed(0, 1, 0);
        e.run(128);
        assert_eq!(e.resource(1).value, 0);
        assert_eq!(e.counts().filtered, 1);
    }
    #[test]
    fn visited_resource_rejects_valid_converging_revisit() {
        let bad = scenario(
            "converging",
            Policy {
                visited: true,
                ..Policy::layered()
            },
        );
        let good = scenario("converging", Policy::layered());
        assert_eq!((bad.resource(0).value, bad.resource(1).value), (1, 2));
        assert_eq!((good.resource(0).value, good.resource(1).value), (4, 4));
        assert_eq!(bad.counts().visited_stops, 1);
        assert!(!good.counts().watchdog);
    }
    #[test]
    fn budgets_bound_oscillation_without_claiming_convergence() {
        let e = scenario("oscillation", Policy::layered());
        assert!(!e.counts().watchdog);
        assert_eq!(e.counts().budget_stops, 1);
        assert_eq!(e.counts().writes, 17);
    }
    #[test]
    fn hop_budget_does_not_bound_fanout_and_small_work_budget_cuts_valid_chain() {
        let fan = scenario(
            "fanout",
            Policy {
                max_hops: 1,
                ..Policy::naive()
            },
        );
        assert_eq!(fan.counts().writes, 65);
        assert_eq!(fan.counts().ready_peak, 64);
        let cut = scenario(
            "converging",
            Policy {
                max_work: 2,
                ..Policy::layered()
            },
        );
        assert!(cut.resource(0).value < 4);
        assert_eq!(cut.counts().budget_stops, 1);
    }
    #[test]
    fn duplicate_and_restart_increment_require_durable_identity() {
        for name in ["duplicate", "restart"] {
            let good = scenario(name, Policy::layered());
            let bad = scenario(
                name,
                Policy {
                    noop: true,
                    ..Policy::naive()
                },
            );
            assert_eq!(good.resource(1).value, 1);
            assert_eq!(bad.resource(1).value, 2);
            assert_eq!(good.counts().duplicates, 1);
            if name == "restart" {
                assert_eq!(good.restarts, 1);
            }
        }
    }
    #[test]
    fn upstream_survives_bounded_downstream_failure_and_transient_retry() {
        let fail = scenario("failure", Policy::layered());
        assert_eq!(fail.resource(0).value, 1);
        assert_eq!(fail.resource(1).value, 0);
        assert_eq!(fail.counts().failures, 3);
        assert_eq!(fail.counts().dead_letters, 1);
        assert_eq!(fail.counts().ticks, 3);
        let transient = scenario("transient", Policy::layered());
        assert_eq!(transient.resource(1).value, 1);
        assert_eq!(transient.counts().failures, 2);
        assert_eq!(transient.counts().dead_letters, 0);
        let permanent = scenario("permanent", Policy::layered());
        assert_eq!(permanent.counts().failures, 1);
        assert_eq!(permanent.counts().dead_letters, 1);
    }
    #[test]
    fn admission_bounds_ready_queue_without_dropping_durable_backlog() {
        let e = scenario("fanout", Policy::layered());
        assert_eq!(e.counts().ready_peak, 4);
        assert_eq!(e.counts().backlog_peak, 64);
        assert_eq!(e.counts().writes, 65);
        assert_eq!(e.pending(), 0);
    }
    #[test]
    fn total_work_caps_fanout_independent_of_depth() {
        let e = scenario(
            "fanout",
            Policy {
                max_work: 12,
                ..Policy::layered()
            },
        );
        assert_eq!(e.counts().writes, 13);
        assert_eq!(e.counts().budget_stops, 52);
    }
    #[test]
    fn stale_projection_requires_revision_guard_even_with_distinct_values() {
        let input = [(2, 20), (1, 10)];
        assert_eq!(latest_projection(&input, false, false), (10, 2));
        assert_eq!(latest_projection(&input, true, false), (20, 1));
    }
    #[test]
    fn safe_projection_coalescing_and_unsafe_domain_event_coalescing() {
        let input = [(1, 10), (2, 20), (3, 30)];
        assert_eq!(latest_projection(&input, true, false), (30, 3));
        assert_eq!(latest_projection(&input, true, true), (30, 1));
        assert_eq!(domain_total(&input, false), 60);
        assert_eq!(domain_total(&input, true), 30);
        assert_eq!(domain_total(&[(1, 10), (1, 10), (2, 20)], false), 30);
    }
    #[test]
    fn monotone_finite_union_converges_despite_graph_cycle() {
        let (set, steps) = monotone_closure(4, &[(0, 1), (1, 2), (2, 0), (2, 3)], 0);
        assert_eq!(set, BTreeSet::from([0, 1, 2, 3]));
        assert_eq!(steps, 4);
    }
    #[test]
    fn queue_bound_and_all_semantic_guards_still_allow_oscillation() {
        for p in [
            Policy {
                queue_cap: 4,
                ..Policy::naive()
            },
            Policy {
                max_hops: usize::MAX,
                max_work: usize::MAX,
                ..Policy::layered()
            },
        ] {
            let e = scenario("oscillation", p);
            assert!(e.counts().watchdog);
            assert_eq!(e.counts().ready_peak, 1);
            assert_eq!(e.counts().writes, 129);
        }
    }
    #[test]
    fn monotone_union_cannot_retract_previously_derived_facts() {
        let edges = [(0, 1), (1, 2), (2, 0), (2, 3)];
        let (old, _) = monotone_closure(4, &edges, 0);
        let (recomputed, _) = monotone_closure(4, &edges, 3);
        let naive_incremental: BTreeSet<_> = old.union(&recomputed).copied().collect();
        assert_eq!(recomputed, BTreeSet::from([3]));
        assert_ne!(naive_incremental, recomputed);
    }
    #[test]
    fn modeled_restart_preserves_root_work_budget() {
        let mut e = Engine::new(
            3,
            vec![rule(0, 1, Transform::Copy), rule(1, 2, Transform::Copy)],
            Policy {
                max_work: 1,
                ..Policy::layered()
            },
        );
        e.seed(0, 1, 0);
        e.crash_after_next_commit();
        e.run(128);
        assert_eq!(e.restarts, 1);
        assert_eq!(e.resource(1).value, 1);
        assert_eq!(e.resource(2).value, 0);
        assert_eq!(e.counts().duplicates, 1);
        assert_eq!(e.counts().budget_stops, 1);
    }
    #[test]
    fn reordered_domain_history_is_recovered_in_order_not_skipped_as_stale() {
        assert_eq!(
            ordered_history(&[(2, 20), (1, 10), (1, 10)]),
            Ok(vec![10, 20])
        );
        assert_eq!(ordered_history(&[(2, 20)]), Err("gap"));
        assert_eq!(
            ordered_history(&[(1, 10), (1, 99)]),
            Err("identity_conflict")
        );
    }
}
