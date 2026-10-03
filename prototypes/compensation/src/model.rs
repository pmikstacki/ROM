//! Throwaway domain declarations: compensation is an ordinary, explicit action.
use rom::{
    Action, Actor, Channel, Definition, Error, Input, PrincipalKind, Reaction, Resource, Result,
    Snapshot, Target,
};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
pub fn author() -> Actor {
    Actor::trusted("prototype", "author")
}
pub fn compensator() -> Actor {
    Actor::trusted("prototype", "compensator").with_kind(PrincipalKind::Service)
}
pub fn provider() -> Actor {
    Actor::trusted("prototype", "provider").with_kind(PrincipalKind::Service)
}
fn policy<R: Resource>(actor: &Actor, _: rom::Access, _: &R) -> bool {
    actor.authority == "prototype"
        && matches!(
            actor.subject.as_str(),
            "author" | "compensator" | "provider"
        )
}
pub fn definition<R: Resource>() -> Definition<R> {
    R::definition().policy(policy::<R>).allow_all_fields()
}

#[derive(Clone, Debug, Resource)]
#[resource(name = "inventory")]
pub struct Inventory {
    pub total: u64,
    pub reservations: BTreeMap<String, u64>,
    pub released: Vec<String>,
}
impl Inventory {
    pub fn available(&self) -> u64 {
        self.total - self.reservations.values().sum::<u64>()
    }
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Reserve {
    pub token: String,
    pub quantity: u64,
}
impl Input for Reserve {
    fn encode(&self) -> serde_json::Value {
        serde_json::to_value(self).unwrap()
    }
    fn decode(value: serde_json::Value) -> Result<Self> {
        serde_json::from_value(value).map_err(|_| Error::invalid("reserve", "input"))
    }
}
pub const RESERVE: Action<Inventory, Reserve> = Action::new("reserve", |stock, input| {
    if input.quantity == 0
        || input.token.is_empty()
        || stock.reservations.contains_key(&input.token)
        || input.quantity > stock.available()
    {
        return Err(Error::Conflict);
    }
    stock.reservations.insert(input.token, input.quantity);
    Ok(vec![])
});
pub const RESTOCK: Action<Inventory, u64> = Action::new("restock", |stock, amount| {
    stock.total = stock.total.checked_add(amount).ok_or(Error::TooLarge)?;
    Ok(vec![])
});
pub const RELEASE: Action<Inventory, String> =
    Action::new("release_own_reservation", |stock, token| {
        if stock.reservations.remove(&token).is_some() {
            stock.released.push(token);
        }
        Ok(vec![])
    });

#[derive(Clone, Debug, Resource)]
#[resource(name = "seating")]
pub struct Seating {
    pub seats: BTreeMap<String, String>,
    pub cancelled: Vec<String>,
}
pub const BOOK: Action<Seating, BTreeMap<String, String>> =
    Action::new("book", |seating, booking| {
        if booking.len() != 1 || booking.keys().any(|seat| seating.seats.contains_key(seat)) {
            return Err(Error::Conflict);
        }
        seating.seats.extend(booking);
        Ok(vec![])
    });
pub const CANCEL: Action<Seating, String> = Action::new("cancel_own_booking", |seating, token| {
    let before = seating.seats.len();
    seating.seats.retain(|_, owner| owner != &token);
    if seating.seats.len() != before {
        seating.cancelled.push(token);
    }
    Ok(vec![])
});

#[derive(Clone, Debug, Resource)]
#[resource(name = "accounts")]
pub struct Account {
    pub owner: String,
    pub enabled: bool,
    pub note: String,
    pub audit: Vec<String>,
}
pub const PROVISION: Action<Account, ()> = Action::new("provision", |account, ()| {
    account.enabled = true;
    account.audit.push("provisioned".into());
    Ok(vec![])
});
pub const ANNOTATE: Action<Account, String> = Action::new("annotate", |account, note| {
    account.note = note;
    account.audit.push("independent_annotation".into());
    Ok(vec![])
});
pub const DISABLE: Action<Account, String> =
    Action::new("disable_owned_account", |account, owner| {
        if account.owner != owner {
            return Err(Error::Conflict);
        }
        if account.enabled {
            account.enabled = false;
            account.audit.push("compensating_disable".into());
        }
        Ok(vec![])
    });

#[derive(Clone, Debug, Resource)]
#[resource(name = "workflows")]
pub struct Workflow {
    pub domain: String,
    pub target: String,
    pub token: String,
    pub outcome: String,
    pub compensate: bool,
    pub decisions: u64,
    pub dispatches: u64,
}
impl Workflow {
    pub fn new(domain: &str, target: &str, enabled: bool) -> Self {
        Self {
            domain: domain.into(),
            target: target.into(),
            token: "flow-a".into(),
            outcome: "pending".into(),
            compensate: enabled,
            decisions: 0,
            dispatches: 0,
        }
    }
}
pub const OBSERVE: Action<Workflow, String> =
    Action::new("record_simulated_business_outcome", |flow, outcome| {
        if !matches!(
            outcome.as_str(),
            "transient" | "unknown" | "confirmed_permanent" | "succeeded"
        ) {
            return Err(Error::invalid("workflow", "outcome"));
        }
        flow.outcome = outcome;
        flow.decisions += 1;
        Ok(vec![])
    });
pub const EXTERNAL: Channel<String> = Channel::new("simulated-provider", 1);
pub const DISPATCH: Action<Workflow, String> =
    Action::new("dispatch_simulated_request", |flow, id| {
        flow.dispatches += 1;
        Ok(vec![EXTERNAL.intent(id)])
    });
fn selected<'a>(flow: &'a Snapshot<Workflow>, domain: &str) -> Option<&'a Workflow> {
    flow.value
        .as_ref()
        .filter(|f| f.domain == domain && f.compensate && f.outcome == "confirmed_permanent")
}
pub fn release(flow: &Snapshot<Workflow>) -> Result<Vec<Target<String>>> {
    Ok(selected(flow, "stock")
        .map(|f| vec![Target::new(&f.target, f.token.clone())])
        .unwrap_or_default())
}
pub fn cancel(flow: &Snapshot<Workflow>) -> Result<Vec<Target<String>>> {
    Ok(selected(flow, "seat")
        .map(|f| vec![Target::new(&f.target, f.token.clone())])
        .unwrap_or_default())
}
pub fn disable(flow: &Snapshot<Workflow>) -> Result<Vec<Target<String>>> {
    Ok(selected(flow, "account")
        .map(|f| vec![Target::new(&f.target, f.token.clone())])
        .unwrap_or_default())
}

#[derive(Clone, Debug, Resource)]
#[resource(name = "recovery_loop")]
pub struct RecoveryLoop {
    pub armed: bool,
    pub phase: bool,
    pub count: u64,
}
pub const ARM: Action<RecoveryLoop, ()> = Action::new("arm_bad_recovery_loop", |value, ()| {
    value.armed = true;
    value.phase = true;
    Ok(vec![])
});
pub const FLIP: Action<RecoveryLoop, ()> = Action::new("bad_compensation_cycle", |value, ()| {
    value.phase = !value.phase;
    value.count += 1;
    Ok(vec![])
});
fn arm(flow: &Snapshot<Workflow>) -> Result<Vec<Target<()>>> {
    Ok(selected(flow, "cycle")
        .map(|f| vec![Target::new(&f.target, ())])
        .unwrap_or_default())
}
fn cycle(state: &Snapshot<RecoveryLoop>) -> Result<Vec<Target<()>>> {
    Ok(state
        .value
        .as_ref()
        .filter(|v| v.armed)
        .map(|_| vec![Target::new(&state.id, ())])
        .unwrap_or_default())
}

pub fn registry() -> rom::Builder {
    rom::Runtime::builder()
        .resource(
            definition::<Inventory>()
                .action(RESERVE)
                .action(RESTOCK)
                .action(RELEASE),
        )
        .resource(definition::<Seating>().action(BOOK).action(CANCEL))
        .resource(
            definition::<Account>()
                .action(PROVISION)
                .action(ANNOTATE)
                .action(DISABLE),
        )
        .resource(definition::<Workflow>().action(OBSERVE).action(DISPATCH))
        .resource(definition::<RecoveryLoop>().action(ARM).action(FLIP))
        .reaction(
            Reaction::new("stock-compensation", 1, compensator(), RELEASE, release)
                .depends_on(Workflow::outcome_field()),
        )
        .reaction(
            Reaction::new("seat-compensation", 1, compensator(), CANCEL, cancel)
                .depends_on(Workflow::outcome_field()),
        )
        .reaction(
            Reaction::new("account-compensation", 1, compensator(), DISABLE, disable)
                .depends_on(Workflow::outcome_field()),
        )
        .reaction(
            Reaction::new("arm-cycle", 1, compensator(), ARM, arm)
                .depends_on(Workflow::outcome_field()),
        )
        .reaction(
            Reaction::new("cycle", 1, compensator(), FLIP, cycle)
                .depends_on(RecoveryLoop::phase_field()),
        )
}
