//! Coroutine Control Block structures for more control.
//!

use crate::{executor::Executor, waker};
use alloc::{boxed::Box, sync::Arc};
use core::{
    future::Future,
    pin::Pin,
    ptr::NonNull,
    sync::atomic::{AtomicU32, Ordering},
    task::{Context, Poll},
};
use crossbeam::atomic::AtomicCell;

/// The pointer of `Task`
#[repr(transparent)]
#[derive(Clone, Copy)]
pub struct TaskRef {
    ptr: NonNull<Task>,
}

unsafe impl Send for TaskRef where &'static Task: Send {}
unsafe impl Sync for TaskRef where &'static Task: Sync {}

impl TaskRef {
    /// Safety: The pointer must have been obtained with `Task::as_ptr`
    pub(crate) unsafe fn from_ptr(ptr: *const Task) -> Self {
        Self {
            ptr: NonNull::new_unchecked(ptr as *mut Task),
        }
    }

    /// The returned pointer
    pub(crate) fn as_ptr(self) -> *const Task {
        self.ptr.as_ptr()
    }
}

///
#[repr(C)]
#[derive(Debug, PartialEq, Eq)]
pub enum TaskType {
    ///
    KernelSche,
    ///
    Other,
}

/// The `Task` is stored in heap by using `Arc`.
#[repr(C)]
pub struct Task {
    pub(crate) executor: AtomicCell<Option<&'static Executor>>,
    ///
    pub priority: AtomicU32,
    ///
    pub task_type: TaskType,
    /// 
    pub fut: AtomicCell<Box<dyn Future<Output = i32> + 'static + Send + Sync>>,
}

impl Task {
    /// Create a new Task, in not-spawned state.
    pub fn new(
        fut: Box<dyn Future<Output = i32> + 'static + Send + Sync>,
        priority: u32,
        task_type: TaskType
    ) -> Arc<Self> {
        Arc::new(Self {
            executor: AtomicCell::new(None),
            priority: AtomicU32::new(priority),
            task_type,
            fut: AtomicCell::new(fut),
        })
    }

    /// Update priority
    pub fn update_priority(&self, new_priority: u32) {
        self.priority.store(new_priority, Ordering::Relaxed);
    }

    ///
    pub fn execute(self: Arc<Self>) -> Poll<i32> {
        unsafe {
            let waker = waker::from_task(self.clone());
            let mut cx = Context::from_waker(&waker);
            let fut = &mut *self.fut.as_ptr();
            let mut future = Pin::new_unchecked(fut.as_mut());
            future.as_mut().poll(&mut cx)
        }
    }
}

/// Wake a task by `TaskRef`.
///
/// You can obtain a `TaskRef` from a `Waker` using [`task_from_waker`].
pub fn wake_task(task_ref: TaskRef) {
    unsafe {
        let task_ptr = task_ref.as_ptr();
        let executor = task_ptr as *const usize as *const Executor;
        (&*executor).wake_task_from_ref(task_ref)
    }
}
