//! This mod specific the waker related with coroutine
//!

use super::task::{wake_task, Task, TaskRef};
use alloc::sync::Arc;
use core::task::{RawWaker, RawWakerVTable, Waker};

const VTABLE: RawWakerVTable = RawWakerVTable::new(clone, wake, wake, drop);

unsafe fn clone(p: *const ()) -> RawWaker {
    RawWaker::new(p, &VTABLE)
}

unsafe fn wake(p: *const ()) {
    wake_task(TaskRef::from_ptr(p as *const Task))
}

unsafe fn drop(p: *const ()) {
    // nop
    let _task = Arc::from_raw(p as *mut Task);
    #[cfg(test)]
    println!("count {}", Arc::strong_count(&_task));
}

/// 
pub unsafe fn from_task(task: Arc<Task>) -> Waker {
    Waker::from_raw(RawWaker::new(Arc::into_raw(task) as _, &VTABLE))
}
