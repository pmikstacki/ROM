//! External application consumer: declarations and business functions, no repositories/controllers.
mod application;
mod setting;
mod task;

pub use application::declarations;
pub use setting::{ENABLE, Setting, setting_policy};
pub use task::{COMPLETE, Task, task_policy};
