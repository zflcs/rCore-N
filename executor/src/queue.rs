/// This mod defines some queue in the `Executor`
use crate::{Task, TaskRef};

use alloc::sync::Arc;
use crossbeam::queue::SegQueue;
use heapless::mpmc::MpMcQueue;

/// This queue will store the task pointers which are waked in kernel.
#[repr(transparent)]
pub struct WakeQueue(MpMcQueue<TaskRef, 128>);

impl WakeQueue {
    ///
    pub const fn new() -> Self {
        Self(MpMcQueue::new())
    }
    ///
    pub fn enqueue(&self, task_ref: TaskRef) {
        let _ = self.0.enqueue(task_ref);
    }
    ///
    pub fn dequeue(&self) -> Option<TaskRef> {
        self.0.dequeue()
    }
}

/// This queue stores the `Arc<Task>` which is ready to run.
#[repr(transparent)]
pub struct RunQueue(SegQueue<Arc<Task>>);

impl RunQueue {
    pub const EMPTY: Self = Self(SegQueue::new());
    ///
    #[allow(unused)]
    pub const fn new() -> Self {
        Self(SegQueue::new())
    }

    ///
    pub fn dequeue(&self) -> Option<Arc<Task>> {
        self.0.pop()
    }

    ///
    pub fn enqueue(&self, task: Arc<Task>) {
        self.0.push(task);
    }
}
