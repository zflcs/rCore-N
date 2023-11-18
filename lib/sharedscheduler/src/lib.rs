//! This crate provides the runtime of sharedscheduler
//!

#![no_std]
#![no_main]

#[macro_use]
extern crate lang;
extern crate alloc;

use core::{future::Future, task::Poll};

use alloc::{boxed::Box, vec::Vec};
use executor::{Executor, Task};
use alloc::vec;
core::arch::global_asm!(include_str!("module_info.asm"));

#[no_mangle]
pub fn test() -> i32 {
    // let mut a = vec![];
    // a.push(34);
    // a.push(35);

    // a.push(36);
    // a.push(37);
    // a.push(345);


    // *a.last().unwrap()
    unsafe { executor_ptr as _ }
}

extern "C" {
    fn executor_ptr();
    fn main() -> i32;
}

#[no_mangle]
#[link_section = ".text.entry"]
pub unsafe fn entry() {
    let main_fut = async { main() };
    spawn(Box::new(main_fut), 0);
    poll_future();
}

#[no_mangle]
pub fn spawn(fut: Box<dyn Future<Output = i32> + 'static + Send + Sync>, priority: u32) {
    let task = Task::new(fut, priority);
    unsafe {
        let exe_ptr = executor_ptr as usize as *mut Executor;
        (*exe_ptr).spawn(task);
    }
}

#[no_mangle]
pub fn poll_future() {
    let executor = unsafe { &mut *(executor_ptr as usize as *mut Executor) };
    if let Some(task) = executor.fetch(0) {
        match task.execute() {
            Poll::Ready(_) => println!("task ready"),
            Poll::Pending => println!("task pending"),
        }
    }
}
