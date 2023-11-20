use core::sync::atomic::{AtomicU32, Ordering};

use alloc::sync::Arc;

use crate::{queue::*, Task, TaskRef, PRIO_LEVEL};
use heapless::{FnvIndexSet, Vec};

/// The `Executor` of `async` runtime.
#[repr(C)]
pub struct Executor {
    /// this queue uses `FIFO` scheduling mechanism no matter what priority the inner task is.
    /// Once there are tasks in this queue, all the tasks in `RunQueue` should be executed later.
    wake_queue: WakeQueue,
    /// The priority will be updated in these situations:
    /// - spawn_task: fetch_min.
    /// - fetch: it will be set as the priority of task which is fetched now.
    /// - wake: fetch_min.
    priority: AtomicU32,
    /// these queues store tasks according to their priority.
    run_queue: [RunQueue; PRIO_LEVEL],
    /// this set stores the pending tasks.
    pending_set: FnvIndexSet<TaskRef, 32>,
    /// current task
    currents: [Option<Arc<Task>>; 4],
    /// thread ids
    threads: Vec<usize, 4>,
}

impl Executor {
    ///
    pub fn new() -> Self {
        Self {
            wake_queue: WakeQueue::new(),
            run_queue: [RunQueue::EMPTY; 8],
            pending_set: FnvIndexSet::new(),
            currents: array_init::array_init(|_| None),
            threads: Vec::new(),
            priority: AtomicU32::new(u32::MAX),
        }
    }

    /// wake a task according to it's pointer
    pub fn wake_task_from_ref(&self, task_ref: TaskRef) {
        let task = unsafe { &*task_ref.as_ptr() };
        let priority = task.priority.load(Ordering::Relaxed);
        self.priority.fetch_min(priority, Ordering::Relaxed);
        self.wake_queue.enqueue(task_ref);
    }

    /// spawn a new task in `Executor`
    pub fn spawn(&'static self, task: Arc<Task>) {
        task.executor.store(Some(self));
        let priority = task.priority.load(Ordering::Relaxed);
        self.run_queue[priority as usize].enqueue(task);
        self.priority.fetch_min(priority, Ordering::Relaxed);
    }

    /// fetch task which has the highest priority
    pub fn fetch(&mut self, tid: usize) -> Option<Arc<Task>> {
        assert!(tid < 4);
        if let Some(task_ref) = self.wake_queue.dequeue() {
            let task = unsafe { Arc::from_raw(task_ref.as_ptr()) };
            let priority = task.priority.load(Ordering::Relaxed);
            self.priority.fetch_min(priority, Ordering::Relaxed);
            self.currents[tid] = Some(task.clone());
            return Some(task);
        }
        let mut task = None;
        for q in &self.run_queue {
            if let Some(t) = q.dequeue() {
                let priority = t.priority.load(Ordering::Relaxed);
                self.priority.fetch_min(priority, Ordering::Relaxed);
                self.currents[tid] = Some(t.clone());
                task = Some(t);
                break;
            }
        }
        task
    }
    ///
    pub fn wake(&mut self, task: Arc<Task>) {
        let priority = task.priority.load(Ordering::Relaxed);
        self.run_queue[priority as usize].enqueue(task);
        self.priority.fetch_min(priority, Ordering::Relaxed);
    }
}
