#![no_std]
#![no_main]


use executor::Task;


#[rcoren::main]
pub async fn main() -> i32 {
    let task = Task::new(Box::new(test(23)), 0, executor::TaskType::Other);
    unsafe {
        let raw_task = Arc::into_raw(task.clone()) as usize;
        let exec_fn = execute as usize;
        core::arch::asm!(
            "jalr t0",
            in("a0") raw_task,
            in("t0") exec_fn,
        );
        let a = 1 + 2;
        println!("back {}", a);
        let raw_task = Arc::into_raw(task.clone()) as usize;
        core::arch::asm!(
            "jalr t0",
            in("a0") raw_task,
            in("t0") exec_fn,
        );
    }    
    0
}

async fn test(a: usize) -> i32 {
    let mut help = Box::new(Help(false));
    help.as_mut().await;
    println!("into test {}", a);
    0
}

struct Help(bool);

impl Future for Help {
    type Output = i32;
    fn poll(mut self: Pin<&mut Self>, _cx: &mut Context<'_>) -> core::task::Poll<Self::Output> {
        if !self.0 {
            self.0 = true;
            core::task::Poll::Pending
        } else {
            core::task::Poll::Ready(0)
        }
    }
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
        match future.as_mut().poll(&mut cx) {
            core::task::Poll::Ready(_) => println!("ok"),
            core::task::Poll::Pending => println!("pending"),
        }
    }
    // sys_exit(0);
}