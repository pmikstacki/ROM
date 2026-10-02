use rom_reaction_chains::{policies, scenario};
fn main() {
    println!(
        "scenario,policy,action_attempts,resource_writes,events,modeled_db_roundtrips,redundant_modeled_roundtrips,noops,duplicates,filtered,budget_stops,visited_stops,failures,dead_letters,ready_peak,backlog_peak,logical_ticks,watchdog,pending,a,b"
    );
    for name in [
        "acyclic",
        "echo",
        "oscillation",
        "converging",
        "unrelated",
        "duplicate",
        "restart",
        "failure",
        "transient",
        "permanent",
        "fanout",
    ] {
        for (label, policy) in policies() {
            let e = scenario(name, policy);
            let c = e.counts();
            println!(
                "{name},{label},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{}",
                c.attempts,
                c.writes,
                c.events,
                c.roundtrips(),
                c.redundant_roundtrips,
                c.noops,
                c.duplicates,
                c.filtered,
                c.budget_stops,
                c.visited_stops,
                c.failures,
                c.dead_letters,
                c.ready_peak,
                c.backlog_peak,
                c.ticks,
                c.watchdog,
                e.pending(),
                e.resource(0).value,
                e.resource(1).value
            );
        }
    }
}
