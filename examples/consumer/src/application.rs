//! Compose the consumer resource definitions.
use crate::{COMPLETE, ENABLE, Setting, Task, setting_policy, task_policy};
use rom::Resource;

pub fn declarations() -> rom::Builder {
    rom::Runtime::builder()
        .resource(
            Task::definition()
                .allow_all_fields()
                .policy(task_policy)
                .action(COMPLETE),
        )
        .resource(
            Setting::definition()
                .allow_all_fields()
                .policy(setting_policy)
                .action(ENABLE),
        )
}
