#![no_std]
#![feature(linkage)]
#![feature(panic_info_message)]
#![feature(alloc_error_handler)]

#[macro_use]
pub mod console;
#[macro_use]
extern crate syscall;
mod lang_items;
pub mod trace;
pub mod trap;
pub mod user_uart;
pub mod matrix;

extern crate alloc;
use alloc::boxed::Box;
pub use syscall::*;
mod heap;
use riscv::register::mtvec::TrapMode;
use riscv::register::{uie, utvec};
use alloc::vec::Vec;

pub use trap::{UserTrapContext, UserTrapQueue, UserTrapRecord};

#[alloc_error_handler]
pub fn handle_alloc_error(layout: core::alloc::Layout) -> ! {
    panic!("Heap allocation error, layout = {:?}", layout);
}

#[no_mangle]
#[link_section = ".text.entry"]
pub extern "C" fn _start(argc: usize, argv: usize) {
    extern "C" {
        fn __alltraps_u();
    }
    unsafe {
        utvec::write(__alltraps_u as usize, TrapMode::Direct);
    }
    heap::init();
    let mut v: Vec<&'static str> = Vec::new();
    for i in 0..argc {
        let str_start =
            unsafe { ((argv + i * core::mem::size_of::<usize>()) as *const usize).read_volatile() };
        let len = (0usize..)
            .find(|i| unsafe { ((str_start + *i) as *const u8).read_volatile() == 0 })
            .unwrap();
        v.push(
            core::str::from_utf8(unsafe {
                core::slice::from_raw_parts(str_start as *const u8, len)
            })
            .unwrap(),
        );
    }
    println!("{:#x?} {:#x?}", argc, argv);
    use basic::FutureFFI;
    let mut main_future = FutureFFI{
        future: Box::pin(async move { main(argc, v); })
    };
    vdso::spawn(&mut main_future, config::PRIO_NUM - 1, basic::CoroutineKind::UserNorm);
}


// 当前正在运行的协程，只能在协程内部使用，即在 async 块内使用
pub fn current_cid() -> usize {
    vdso::current_cid(false)
}

pub fn re_back(cid: usize) {
    let pid = getpid() as usize;
    vdso::re_back(cid, pid + 1);
}

pub fn add_virtual_core() {
    vdso::add_virtual_core();
}

#[linkage = "weak"]
#[no_mangle]
fn main(_argc: usize, _argv: Vec<&'static str>) -> i32 {
    panic!("Cannot find main!");
}

pub fn init_user_trap() -> isize {
    let tid = thread_create(user_interrupt_handler as usize, 0);
    let ans = sys_init_user_trap(tid as usize);
    ans
}

fn user_interrupt_handler() {
    extern "C" {
        fn __alltraps_u();
    }
    unsafe {
        utvec::write(__alltraps_u as usize, TrapMode::Direct);
        uie::set_usoft();
        uie::set_utimer();
    }

    loop {
        hang();
    }
}
