//! This crate provides the runtime of sharedscheduler
//!

#![no_std]
#![no_main]

#[macro_use]
extern crate lang;
extern crate alloc;

use core::{future::Future, task::Poll};

use alloc::boxed::Box;
use executor::{Executor, Task};
core::arch::global_asm!(include_str!("module_info.asm"));


extern "C" {
    fn executor_ptr();
    #[allow(improper_ctypes)]
    fn main() -> Box<dyn Future<Output = i32> + 'static + Send + Sync>;
}

#[no_mangle]
#[link_section = ".text.entry"]
pub unsafe fn entry() {
    let main_fut = main();
    spawn(main_fut, 0);
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
    while let Some(task) = executor.fetch(0) {
        match task.execute() {
            Poll::Ready(_) => println!("task ready"),
            Poll::Pending => println!("task pending"),
        }
    }
}
