#![no_std]
#![no_main]
#![feature(panic_info_message)]
#![feature(alloc_error_handler)]
#![feature(map_try_insert)]
#![feature(vec_into_raw_parts)]
#![allow(unused)]


extern crate alloc;
extern crate rv_plic;

#[macro_use]
extern crate bitflags;
#[macro_use]
extern crate log;

use alloc::boxed::Box;
use config::CPU_NUM;
use mm::init_kernel_space;
use sbi::send_ipi;
use core::{arch::{asm, global_asm}, pin::Pin, ffi::c_void};

use crate::{mm::KERNEL_SPACE, lkm::{LKM_MANAGER, ModuleManager}, loader::get_app_data_by_name, task::INITPROC};

#[macro_use]
mod console;
#[macro_use]
mod fs;
mod lang_items;
mod plic;
mod sbi;
mod syscall;
mod task;
mod sync;
mod timer;
mod trap;
#[macro_use]
mod uart;
mod err;
mod mm;
mod logger;
mod loader;
mod lkm;

global_asm!(include_str!("entry.asm"));
global_asm!(include_str!("link_app.asm"));
global_asm!(include_str!("symbol_table.asm"));

fn clear_bss() {
    extern "C" {
        fn sbss();
        fn ebss();
    }
    (sbss as usize..ebss as usize).for_each(|a| unsafe { (a as *mut u8).write_volatile(0) });
}

#[inline]
pub fn hart_id() -> usize {
    let hart_id: usize;
    unsafe {
        asm!("mv {}, tp", out(reg) hart_id);
    }
    hart_id
}

#[no_mangle]
pub fn rust_main(hart_id: usize) -> ! {
    if hart_id == 0 {
        clear_bss();
        logger::init();
        mm::init();
        mm::remap_test();
        // KERNEL_SPACE.lock().pmap();
        trap::init();
        plic::init();
        plic::init_hart(hart_id);
        uart::init();

        extern "C" {
            fn boot_stack();
            fn boot_stack_top();
        }

        debug!("boot_stack {:#x} top {:#x}", boot_stack as usize, boot_stack_top as usize);
        // unsafe {
        //     let satp: usize;
        //     let sp: usize;
        //     asm!("csrr {}, satp", out(reg) satp);
        //     asm!("mv {}, sp", out(reg) sp);
        //     println_hart!("satp: {:#x}, sp: {:#x}", hart_id, satp, sp);
        // }
        ModuleManager::init();
        LKM_MANAGER.lock().as_mut().unwrap().init_module("sharedscheduler", "");
        lkm::init();
        debug!("trying to add initproc");
        task::add_initproc();
        debug!("initproc added to task manager!");
        // INITPROC.acquire_inner_lock().memory_set.pmap();
        loader::list_apps();
        for i in 1..CPU_NUM {
            debug!("[kernel {}] Start {}", hart_id, i);
            let mask: usize = 1 << i;
            send_ipi(&mask as *const _ as usize);
        }
    } else {
        let hart_id = task::hart_id();
        init_kernel_space();
        unsafe {
            let satp: usize;
            let sp: usize;
            asm!("csrr {}, satp", out(reg) satp);
            asm!("mv {}, sp", out(reg) sp);
            println_hart!("satp: {:#x}, sp: {:#x}", hart_id, satp, sp);
        }
        trap::init();
        plic::init_hart(hart_id);
    }
    timer::set_next_trigger();
    vdso::spawn(move || task::run_tasks(), 7, 0, basic::CoroutineKind::KernSche);
    vdso::poll_kernel_future();
    panic!("Unreachable in rust_main!");
}
