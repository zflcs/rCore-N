mod trampoline;
mod trapframe;

use core::{arch::asm, panic};
use log::*;
use riscv::register::{scause::*, utvec::TrapMode, *};
pub use trampoline::__trampoline;
pub use trapframe::TrapFrame;

use crate::{
    arch::mm::VirtAddr,
    config::TRAMPOLINE_VA,
    error::KernelError,
    mm::{do_handle_page_fault, VMFlags},
    println,
    syscall::syscall,
    task::*,
    timer::set_next_trigger,
};
#[cfg(feature = "board_axu15eg")]
use crate::driver::plic;

use self::trapframe::KernelTrapContext;

use crate::arch::uintr::*;

/// Set kernel trap entry.
pub fn set_kernel_trap() {
    extern "C" {
        fn __kernelvec();
    }
    unsafe {
        stvec::write(
            __kernelvec as usize - __trampoline as usize + TRAMPOLINE_VA,
            TrapMode::Direct,
        );
        sscratch::write(kernel_trap_handler as usize);
    }
    enable_supervisor_intr();
}

/// Set user trap entry.
pub fn set_user_trap() {
    disable_supervisor_intr();
    unsafe { stvec::write(TRAMPOLINE_VA as usize, TrapMode::Direct) };
}

pub fn enable_timer_intr() {
    unsafe {
        sie::set_stimer();
        // sstatus::set_sie();
    }
}

pub fn enable_ext_intr() {
    unsafe { sie::set_sext(); }
}

pub fn enable_supervisor_intr() {
    unsafe { sstatus::set_sie() }
}

pub fn disable_supervisor_intr() {
    unsafe { sstatus::clear_sie(); }
}

/// User trap handler manages the task according to the cause:
///
/// 1. Calls syscall dispatcher and handler.
/// 2. Handles page fault caused by Instruction Fetch, Load or Store.
#[no_mangle]
pub fn user_trap_handler() -> ! {
    uintr_save();

    set_kernel_trap();

    let scause = scause::read();
    let sstatus = sstatus::read();
    let stval = stval::read();
    let sepc = sepc::read();
    // Only handle user trap
    assert!(sstatus.spp() == sstatus::SPP::User);

    // Handle user trap with detailed cause
    let show_trapframe = |tf: &TrapFrame| {
        println!("{:#X?}", tf);
    };
    let trap_info = || {
        trace!(
            "[U] {:X?}, {:X?}, stval={:#X}, sepc={:#X}",
            scause.cause(),
            sstatus,
            stval,
            sepc,
        )
    };
    let fatal_info = |err: KernelError| {
        trace!("[U] Fatal exception {:#?}", err);
    };

    match scause.cause() {
        Trap::Exception(Exception::UserEnvCall) => {
            // pc + 4
            let curr = cpu().curr.as_ref().unwrap();
            let trapframe = curr.trapframe();
            trapframe.next_epc();

            match syscall(trapframe.syscall_args().unwrap()) {
                Ok(ret) => trapframe.set_a0(ret),
                Err(errno) => {
                    trace!("{:#?} {:#?}", trapframe.syscall_args().unwrap().0, errno);
                    trapframe.set_a0(-isize::from(errno) as usize)
                }
            }
        }
        Trap::Exception(Exception::StorePageFault) => {
            let curr = cpu().curr.as_ref().unwrap();
            let mut curr_mm = curr.mm();
            trap_info();
            if let Err(err) = do_handle_page_fault(
                &mut curr_mm,
                VirtAddr::from(stval),
                VMFlags::USER | VMFlags::WRITE,
            ) {
                fatal_info(err);
                drop(curr_mm);
                unsafe { do_exit(-1) };
            }
        }
        Trap::Interrupt(Interrupt::SupervisorTimer) => {
            trap_info();
            set_next_trigger();
            unsafe { do_yield() };
        }
        #[cfg(feature = "board_axu15eg")]
        Trap::Interrupt(Interrupt::SupervisorExternal) => {
            use crate::arch::get_cpu_id;
            trap_info();
            plic::handle_external_interrupt(get_cpu_id());
        }
        _ => {
            let curr = cpu().curr.as_ref().unwrap();
            show_trapframe(curr.trapframe());
            trap_info();
            unsafe { do_exit(-1) };
        }
    }
    user_trap_return();
}

/// Something prepared before `sret` back to user:
///
/// 1. Set `stvec` to user trap entry again.
/// 2. Jump to raw assembly code, passing the address of trapframe and `satp`.
///
/// # DEAD LOCK
///
/// This function acquires a reference and the lock of address space metadata of
/// current task. We must drop them before changing the control flow without unwinding.
#[no_mangle]
pub fn user_trap_return() -> ! {
    uintr_return();

    extern "C" {
        fn __uservec();
        fn __userret();
    }
    let (satp, trapframe_base, userret) = {
        let curr = cpu().curr.as_ref().unwrap();
        let curr_mm = curr.mm();
        (
            curr_mm.page_table.satp(),
            trapframe_base(curr.tid.0),
            __userret as usize - __uservec as usize + TRAMPOLINE_VA,
        )
    };

    set_user_trap();

    unsafe {
        asm!(
            "jr {userret}",
            userret = in(reg) userret,
            in("a0") trapframe_base,
            in("a1") satp,
            options(noreturn)
        );
    }
}

#[no_mangle]
pub fn kernel_trap_handler(ctx: &KernelTrapContext) {
    let scause = scause::read();
    let stval = stval::read();
    match scause.cause() {
        Trap::Interrupt(Interrupt::SupervisorTimer) => {
            set_next_trigger();
        },
        #[cfg(feature = "board_axu15eg")]
        Trap::Interrupt(Interrupt::SupervisorExternal) => {
            use crate::arch::get_cpu_id;
            plic::handle_external_interrupt(get_cpu_id());
        },
        _ => {
            panic!(
                "[S] {:X?}, stval = {:#X}, ctx = {:#X?} ",
                scause.cause(),
                stval,
                ctx
            );
        }
    };
}
