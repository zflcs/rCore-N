#![no_std]
#![feature(linkage)]
#![feature(panic_info_message)]
#![feature(alloc_error_handler)]
#![feature(naked_functions)]

#[macro_use]
pub mod console;
mod lang_items;
pub mod matrix;
mod uintrtrap;

extern crate alloc;
use core::future::Future;
use core::pin::Pin;
use core::task::{Context, Poll};

pub use time_subsys::*;
mod heap;
pub use user_syscall::*;
pub use uintrtrap::*;

#[alloc_error_handler]
pub fn handle_alloc_error(layout: core::alloc::Layout) -> ! {
    panic!("Heap allocation error, layout = {:?}", layout);
}

#[no_mangle]
#[link_section = ".text.entry"]
pub extern "C" fn _start() {
    heap::init();
    vdso::spawn(move || async{ main(); }, executor::MAX_PRIO - 1, executor::CoroutineKind::Norm);
}


pub struct AwaitHelper {
    flag: bool,
}

impl AwaitHelper {
    pub fn new() -> Self {
        AwaitHelper {
            flag: false,
        }
    }
}

impl Future for AwaitHelper {
    type Output = ();

    fn poll(mut self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Self::Output> {
        if self.flag == false {
            self.flag = true;
            return Poll::Pending;
        }
        return Poll::Ready(());
    }
}



#[linkage = "weak"]
#[no_mangle]
fn main() -> i32 {
    panic!("Cannot find main!");
}

