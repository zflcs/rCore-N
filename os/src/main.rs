#![no_std]
#![no_main]
#![feature(panic_info_message)]
#![feature(alloc_error_handler)]
#![feature(map_try_insert)]
#![feature(vec_into_raw_parts)]
#![feature(new_uninit)]
#![feature(naked_functions)]
#![feature(asm_const)]
#![feature(exact_size_is_empty)]
#![allow(unused)]

extern crate alloc;
extern crate rv_plic;

#[macro_use]
extern crate bitflags;
#[macro_use]
extern crate log;



#[macro_use]
extern crate lang;


mod config;
mod fs;
mod mm;
mod lkm;
// mod sync;
mod syscall;
mod task;
mod timer;
mod trap;

mod device;
// mod net;

// use device::plic;
pub type Result<T> = core::result::Result<T, ()>;

use crate::{config::CPU_NUM, mm::{init_kernel_space, KERNEL_SPACE}, lkm::LKM_MANAGER};
use core::sync::atomic::{AtomicUsize, Ordering};
use alloc::boxed::Box;
use core::future::Future;

core::arch::global_asm!(include_str!("ramfs.asm"));

/// Boot kernel size allocated in `_start` for single CPU.
pub const BOOT_STACK_SIZE: usize = 0x4_0000;

/// Total boot kernel size.
pub const TOTAL_BOOT_STACK_SIZE: usize = BOOT_STACK_SIZE * CPU_NUM;

/// Initialize kernel stack in .bss section.
#[link_section = ".bss.stack"]
static mut STACK: [u8; TOTAL_BOOT_STACK_SIZE] = [0u8; TOTAL_BOOT_STACK_SIZE];

/// Entry for the first kernel.
#[naked]
#[no_mangle]
#[link_section = ".text.entry"]
pub unsafe extern "C" fn _start(hartid: usize) -> ! {
    core::arch::asm!(
        // Use tp to save hartid
        "mv tp, a0",
        // Set stack pointer to the kernel stack.
        "
        la a1, {stack}
        li t0, {total_stack_size}
        li t1, {stack_size}
        mul sp, a0, t1
        sub sp, t0, sp
        add sp, a1, sp
        ",        // Jump to the main function.
        "j  {main}",
        total_stack_size = const TOTAL_BOOT_STACK_SIZE,
        stack_size       = const BOOT_STACK_SIZE,
        stack            =   sym STACK,
        main             =   sym rust_main_init,
        options(noreturn),
    )
}

/// Entry for other kernels.
#[naked]
#[no_mangle]
pub unsafe extern "C" fn __entry_others(hartid: usize) -> ! {
    core::arch::asm!(
        // Use tp to save hartid
        "mv tp, a0",
        // Set stack pointer to the kernel stack.
        "
        la a1, {stack}
        li t0, {total_stack_size}
        li t1, {stack_size}
        mul sp, a0, t1
        sub sp, t0, sp
        add sp, a1, sp
        ",
        // Jump to the main function.
        "j  {main}",
        total_stack_size = const TOTAL_BOOT_STACK_SIZE,
        stack_size       = const BOOT_STACK_SIZE,
        stack            =   sym STACK,
        main             =   sym rust_main_init_other,
        options(noreturn),
    )
}

fn clear_bss() {
    extern "C" {
        fn sbss();
        fn ebss();
    }
    (sbss as usize..ebss as usize).for_each(|a| unsafe { (a as *mut u8).write_volatile(0) });
}

static BOOT_HART: AtomicUsize = AtomicUsize::new(0);

#[no_mangle]
pub fn rust_main_init(hart_id: usize) -> ! {
    clear_bss();
    lang::console::init(option_env!("LOG"));
    mm::init();
    fs::inode::list_apps();
    let _ = LKM_MANAGER.lock();
    BOOT_HART.fetch_add(1, Ordering::Relaxed);
    trap::init();
    // net::init();
    // device::init();
    // plic::init();
    // plic::init_hart(hart_id);
    task::add_initproc();


    // if CPU_NUM > 1 {
    //     for i in 0..CPU_NUM {
    //         let boot_hart_cnt = BOOT_HART.load(Ordering::Relaxed);
    //         if i != hart_id {
    //             debug!("Start {}", i);
    //             // Starts other harts.
    //             let ret = sbi_rt::hart_start(i, __entry_others as _, 0);
    //             assert!(ret.is_ok(), "Failed to shart hart {}", i);
    //             while BOOT_HART.load(Ordering::Relaxed) == boot_hart_cnt {}
    //         }
    //     }
    // }
    rust_main(hart_id)
}

#[no_mangle]
pub fn rust_main_init_other(hart_id: usize) -> ! {
    init_kernel_space();
    trap::init();
    // plic::init_hart(hart_id);
    BOOT_HART.fetch_add(1, Ordering::Relaxed);
    rust_main(hart_id)
}

#[no_mangle]
pub fn rust_main(_hart_id: usize) -> ! {
    // timer::set_next_trigger();
    lkm::spawn(Box::new(task::run_tasks()), 7, executor::TaskType::KernelSche);
    lkm::entry();
    
    panic!("Unreachable in rust_main!");
}

#[no_mangle]
pub fn main() -> Box<dyn Future<Output = i32> + 'static + Send + Sync> {
    log::debug!("into main function");
    Box::new(async {0})
}




#[no_mangle]
fn put_str(ptr: *const u8, len: usize) {
    let bytes = unsafe { core::slice::from_raw_parts(ptr, len) };
    for c in bytes {
        #[allow(deprecated)]
        sbi_rt::legacy::console_putchar(*c as _);
    }
}
