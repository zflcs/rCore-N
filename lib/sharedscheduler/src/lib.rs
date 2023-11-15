//! This crate provides the runtime of sharedscheduler
//! 

#![no_std]
#![no_main]

#[macro_use]
extern crate lib;
extern crate alloc;

use core::{future::Future, task::Poll};

use alloc::{vec, boxed::Box};
use executor::{Executor, Task};

#[no_mangle]
pub fn test() -> i32 {
    let mut _a = vec![1, 3, 4];
    _a.push(2);
    println!("test");
    2
}

extern "C" {
    static EXECUTOR_PTR: usize;
    fn user_main() -> i32;
}



#[no_mangle]
#[link_section = ".text.entry"]
pub unsafe fn user_entry() {
    let main_fut = async { user_main() };
    spawn(Box::new(main_fut), 0);
    poll_future();
}

#[no_mangle]
pub fn spawn(
    fut: Box<dyn Future<Output = i32> + 'static + Send + Sync>, 
    priority: u32
) {
    let task = Task::new(fut, priority);
    unsafe {
        let exe_ptr = EXECUTOR_PTR as *mut Executor;
        (*exe_ptr).spawn(task);
    }
}

#[no_mangle]
pub fn poll_future () {
    let executor = unsafe { &mut *(EXECUTOR_PTR as *mut Executor) };
    if let Some(task) = executor.fetch(0) {
        match task.execute() {
            Poll::Ready(_) => println!("task ready"),
            Poll::Pending => println!("task pending"),
        }
    }

}