#![no_std]
#![feature(linkage)]
#![feature(panic_info_message)]
#![feature(alloc_error_handler)]
#![feature(naked_functions)]

#[macro_use]
pub mod console;
#[macro_use]
extern crate syscall;
mod lang_items;
pub mod matrix;

extern crate alloc;
use core::future::Future;
use core::pin::Pin;
use core::task::{Context, Poll};

pub use syscall::*;
mod heap;

#[alloc_error_handler]
pub fn handle_alloc_error(layout: core::alloc::Layout) -> ! {
    panic!("Heap allocation error, layout = {:?}", layout);
}

#[no_mangle]
#[link_section = ".text.entry"]
pub extern "C" fn _start() {
    // pub extern "C" fn _start(argc: usize, argv: usize) -> ! {

    heap::init();
    lib_so::spawn(
        move || async {
            main();
        },
        lib_so::PRIO_NUM - 1,
        getpid() as usize + 1,
        lib_so::CoroutineKind::UserNorm,
    );
}

// 当前正在运行的协程，只能在协程内部使用，即在 async 块内使用
pub fn current_cid() -> usize {
    lib_so::current_cid(false)
}

pub fn re_back(cid: usize) {
    let pid = getpid() as usize;
    lib_so::re_back(cid, pid + 1);
}

pub fn add_virtual_core() {
    lib_so::add_virtual_core();
}

pub fn spawn<F, T>(f: F, prio: usize) -> usize
where
    F: FnOnce() -> T,
    T: Future<Output = ()> + 'static + Send + Sync,
{
    lib_so::spawn(
        f,
        prio,
        sys_get_pid() as usize + 1,
        lib_so::CoroutineKind::UserNorm,
    )
}

pub fn get_pending_status(cid: usize) -> bool {
    lib_so::get_pending_status(cid)
}

pub struct AwaitHelper {
    flag: bool,
}

impl AwaitHelper {
    pub fn new() -> Self {
        AwaitHelper { flag: false }
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

pub struct TimerHelper {
    interval: usize,
    time: usize,
}

impl TimerHelper {
    pub fn new(interval: usize) -> Self {
        TimerHelper {
            interval,
            time: get_time() as usize,
        }
    }
}

impl Future for TimerHelper {
    type Output = ();

    fn poll(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Self::Output> {
        let cur_time = get_time() as usize;
        if self.time + self.interval > cur_time {
            // println!("send start: {}", current_cid());
            set_timer!(((self.time + self.interval) * 1000) as isize, current_cid());
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
