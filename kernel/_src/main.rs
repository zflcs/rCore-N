#![no_std]
#![no_main]
#![feature(panic_info_message)]
#![feature(alloc_error_handler)]
#![feature(map_try_insert)]
#![feature(vec_into_raw_parts)]
#![allow(unused)]
#![feature(new_uninit)]
#![feature(naked_functions)]
#![feature(asm_const)]

extern crate alloc;
extern crate rv_plic;

#[macro_use]
extern crate bitflags;
#[macro_use]
extern crate log;

use crate::{config::CPU_NUM, mm::init_kernel_space, sbi::send_ipi};
use core::{arch::{asm, global_asm}, sync::atomic::{Ordering::Relaxed, AtomicUsize}};

#[macro_use]
mod console;
mod config;
#[macro_use]
mod fs;
mod lang_items;
mod loader;
mod logger;
mod mm;
mod sbi;
mod syscall;
mod task;
mod sync;
mod timer;
mod trap;
#[macro_use]
mod trace;
mod lkm;
mod device;
mod net;

use alloc::vec;

use device::{plic, uart};

global_asm!(include_str!("link_app.asm"));

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
    if BOOT_HART.load(Relaxed) != 0 {
        return rust_main_init_other(hart_id);
    }
    clear_bss();
    logger::init();
    mm::init();
    BOOT_HART.fetch_add(1, Relaxed);
    mm::remap_test();
    trap::init();
    device::init();
    plic::init();
    plic::init_hart(hart_id);
    uart::init();
    lkm::init();

    debug!("trying to add initproc");
    task::add_initproc();
    debug!("initproc added to task manager!");

    if CPU_NUM > 1 {
        for i in 0..CPU_NUM {
            let boot_hart_cnt = BOOT_HART.load(Relaxed);
            if i != hart_id {
                debug!("Start {}", i);
                // Starts other harts.
                let ret = sbi_rt::hart_start(i, __entry_others as _, 0);
                assert!(ret.is_ok(), "Failed to shart hart {}", i);
                while BOOT_HART.load(Relaxed) == boot_hart_cnt {}
            }
        }
    }
    loader::list_apps();
    rust_main(hart_id)

}

#[no_mangle]
pub fn rust_main_init_other(hart_id: usize) -> !{
    init_kernel_space();
    trap::init();
    plic::init_hart(hart_id);
    BOOT_HART.fetch_add(1, Relaxed);
    rust_main(hart_id)
}

#[no_mangle]
pub fn rust_main(hart_id: usize) -> ! {
    timer::set_next_trigger();
    lib_so::spawn(move || task::run_tasks(), 7, 0, lib_so::CoroutineKind::KernSche);
    lib_so::poll_kernel_future();
    panic!("Unreachable in rust_main!");
}


