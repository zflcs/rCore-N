use alloc::sync::Arc;
use alloc::vec::Vec;

use alloc::boxed::Box;
use core::pin::Pin;
use core::future::Future;

use core::sync::atomic::{AtomicUsize, Ordering::Relaxed};

use crate::config::{MAX_THREAD, MAX_PRIO, PRIO_POINTER};
use crate::coroutine::{Coroutine, CoroutineId, CoroutineKind};
use crossbeam::queue::SegQueue;

pub const EMPTY_QUEUE: SegQueue<Arc<Coroutine>> = SegQueue::new();

/// The highest priority field
#[derive(Clone, Copy)]
struct Priority(usize);

impl Priority {
    pub const DEFAULT: Self = Self(PRIO_POINTER);

    // This is used when spawn or wake up a coroutine
    pub fn update(&self, prio: usize) {
        let priority = unsafe { &*(self.0 as *mut AtomicUsize) };
        priority.fetch_min(prio, Relaxed);
    }

    pub fn set_prio(&self, prio: usize) {
        let priority = unsafe { &*(self.0 as *mut AtomicUsize) };
        priority.store(prio, Relaxed);
    }

    pub fn get_prio(&self) -> usize {
        let priority = unsafe { &*(self.0 as *mut AtomicUsize) };
        priority.load(Relaxed)
    }
}

/// The Executor of a process
pub struct Executor {
    /// Current running coroutine's cid
    pub currents: [Option<Arc<Coroutine>>; MAX_THREAD],
    /// the queue of ready coroutines
    pub ready_queue: [SegQueue<Arc<Coroutine>>; MAX_PRIO],
    /// the set of all pending coroutines
    pub pending_queue: SegQueue<Arc<Coroutine>>,
    /// The highest priority
    priority: Priority,
    /// all theads' id, when it's time to exit, it must wait all threads
    pub waits: Vec<usize>,
}

// unsafe impl Sync for Executor {}
// unsafe impl Send for Executor {}

impl Executor {

    pub const fn new() -> Self {
        Self {
            currents: [const { None }; MAX_THREAD],
            ready_queue: [EMPTY_QUEUE; MAX_PRIO],
            pending_queue: SegQueue::new(),
            priority: Priority::DEFAULT,
            waits: Vec::new(),
        }
    }
}

impl Executor {

    /// add a new coroutine into ready_queue
    pub fn spawn(&mut self, future: Pin<Box<dyn Future<Output=()> + 'static + Send + Sync>>, prio: usize, kind: CoroutineKind) -> CoroutineId {
        let task = Coroutine::new(future, prio, kind);
        let cid = task.cid;
        self.ready_queue[prio].push(task);
        self.priority.update(prio);
        cid
    }

    pub fn add(&mut self, task: Arc<Coroutine>) {
        let prio = task.inner.lock().prio;
        self.ready_queue[prio].push(task);
        self.priority.update(prio);
    }
    
    pub fn is_empty(&self) -> bool {
        for i in 0..MAX_PRIO {
            if !self.ready_queue[i].is_empty() {
                return false;
            }
        }
        self.pending_queue.is_empty()
    }

    /// fetch coroutine which is the highest priority
    pub fn fetch(&mut self, tid: usize) -> Option<Arc<Coroutine>> {
        assert!(tid < MAX_THREAD);
        for i in 0..MAX_PRIO {
            if let Some(task) = self.ready_queue[i].pop() {
                self.currents[tid] = Some(task.clone());
                return Some(task);
            }
        }
        return None;
    }

    /// let pending coroutine into pending queue
    pub fn pending(&mut self, task: Arc<Coroutine>) {
        self.pending_queue.push(task)
    }

    // Check a coroutine is or not pending
    pub fn is_pending(&mut self, cid: usize) -> bool {
        for _ in 0..self.pending_queue.len() {
            let task = self.pending_queue.pop().unwrap();
            let id = task.cid.0;
            self.pending_queue.push(task);
            if id == cid {
                return true;
            }
        }
        false
    }

    /// add a new thread id
    pub fn add_wait_tid(&mut self, tid: usize) {
        self.waits.push(tid);
    }

    /// The pending coroutine 
    pub fn re_back(&mut self, cid: CoroutineId) {
        let mut target_task = None;
        for _ in 0..self.pending_queue.len() {
            let task = self.pending_queue.pop().unwrap();
            if task.cid == cid {
                target_task = Some(task);
                break;
            } else {
                self.pending_queue.push(task);
            }
        };
        if let Some(task) = target_task {
            let prio = task.inner.lock().prio;
            self.ready_queue[prio].push(task);
            self.priority.update(prio);
        }
    }

    /// remove a coroutine
    pub fn remove(&self, task: Arc<Coroutine>) {
        drop(task);
    }

    /// current coroutine id
    pub fn cur(&self, tid: usize) -> CoroutineId {
        assert!(self.currents[tid].is_some());
        self.currents[tid].as_ref().unwrap().cid
    }

    /// update state after a coroutine is executed
    pub fn update_state(&mut self, tid: usize) {
        self.currents[tid] = None;
        for i in 0..MAX_PRIO {
            if !self.ready_queue[i].is_empty() {
                self.priority.set_prio(i);
                return;
            }
        }
        self.priority.set_prio(MAX_PRIO - 1);
    }

    /// get prio
    pub fn get_prio(&self) -> usize {
        self.priority.get_prio()
    }
}