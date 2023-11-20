#![no_std]
#![no_main]


use macros::main;
use executor::Task;


#[main]
pub async fn main() -> i32 {
    let task = Task::new(Box::new(test()), 0, executor::TaskType::Other);
    unsafe {
        let raw_task = Arc::into_raw(task) as usize;
        let exec_fn = execute as usize;
        core::arch::asm!(
            "jr t0",
            in("a0") raw_task,
            in("t0") exec_fn,
        );
    }    
    0
}

async fn test() -> i32 {
    println!("into test");
    0
}

use executor::waker;
use core::task::Context;
use alloc::sync::Arc;
use core::pin::Pin;

pub fn execute(task: *const Task) {
    unsafe {
        let task = Arc::from_raw(task);
        let waker = waker::from_task(task.clone());
        let mut cx = Context::from_waker(&waker);
        let fut = &mut *task.fut.as_ptr();
        let mut future = Pin::new_unchecked(fut.as_mut());
        future.as_mut().poll(&mut cx);
    }
    sys_exit(0);
}