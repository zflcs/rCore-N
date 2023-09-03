mod clone;
mod exit;
mod sched;
mod task;
mod limit;

pub use clone::*;
pub use exit::*;
pub use sched::*;
pub use task::*;
pub use sched::*;
pub use limit::*;

use lazy_static::lazy_static;
use alloc::{sync::Arc, vec, string::String};

lazy_static! {
    pub static ref SHELL: Arc<Task> =
        Arc::new(Task::new(String::from("/"), crate::loader::get_app_data_by_name("shell").unwrap(), vec![String::from("shell"); 1]).unwrap());
}

pub fn add_shell() {
    TASK_MANAGER.lock().add(KernTask::Proc(SHELL.clone()));
}