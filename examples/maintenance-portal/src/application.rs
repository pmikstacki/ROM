use crate::{COMPLETE, Equipment, Inspection, PortalSettings, WorkOrder, policy};

pub fn declarations() -> rom::Builder {
    rom::Runtime::builder()
        .resource(policy::definition::<Equipment>())
        .resource(policy::definition::<Inspection>())
        .resource(policy::definition::<WorkOrder>().action(COMPLETE))
        .resource(policy::definition::<PortalSettings>())
        .resource(crate::catalog::definition())
}
