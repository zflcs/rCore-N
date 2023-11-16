mod context;
pub use context::TrapContext;

use crate::config::TRAMPOLINE;
use crate::plic;
use crate::syscall::syscall;
use crate::task::{
    current_task, current_trap_cx, current_trap_cx_user_va, current_user_token,
    exit_current_and_run_next, hart_id,
};
use crate::timer::get_time_us;
use core::arch::{asm, global_asm};
use riscv::register::scounteren;
use riscv::register::{
    mtvec::TrapMode,
    scause::{self, Exception, Interrupt, Trap},
    sepc, sideleg, sie, sip, sstatus, stval, stvec,
};

global_asm!(include_str!("trap.asm"));

pub fn init() {
    unsafe {
        sie::set_stimer();
        sie::set_sext();
        sie::set_ssoft();
        sideleg::set_usoft();
        sideleg::set_uext();
        sideleg::set_utimer();
        scounteren::set_cy();
        scounteren::set_tm();
        scounteren::set_ir();
    }
    set_kernel_trap_entry();
}

fn set_kernel_trap_entry() {
    unsafe {
        extern "C" {
            fn kernelvec();
        }
        stvec::write(kernelvec as usize, TrapMode::Direct);
    }
}

fn set_user_trap_entry() {
    unsafe {
        stvec::write(TRAMPOLINE as usize, TrapMode::Direct);
    }
}

#[no_mangle]
pub fn trap_handler() -> ! {
    set_kernel_trap_entry();
    let scause = scause::read();
    let stval = stval::read();
    // push_trace(S_TRAP_HANDLER + scause.bits());
    let task = current_task().unwrap();
    let mut inner = task.acquire_inner_lock();
    inner.user_time_us += get_time_us() - inner.last_user_time_us;
    drop(inner);
    match scause.cause() {
        Trap::Exception(Exception::UserEnvCall) => {
            // jump to next instruction anyway
            let cx = current_trap_cx();
            cx.sepc += 4;
            let id = cx.x[17];
            // get system call return value
            let result = syscall(
                cx.x[17],
                [cx.x[10], cx.x[11], cx.x[12], cx.x[13], cx.x[14], cx.x[15]],
            );
            if id != 221 || result != 0 {
                cx.x[10] = result as usize;
            }
        }
        Trap::Exception(Exception::StoreFault)
        | Trap::Exception(Exception::StorePageFault)
        | Trap::Exception(Exception::InstructionFault)
        | Trap::Exception(Exception::InstructionPageFault)
        | Trap::Exception(Exception::LoadFault)
        | Trap::Exception(Exception::LoadPageFault) => {
            error!(
                "[kernel] {:?} in application, bad addr = {:#x}, bad instruction = {:#x}, core dumped.",
                scause.cause(),
                stval,
                current_trap_cx().sepc,
            );
            // page fault exit code
            exit_current_and_run_next(-2);
        }
        Trap::Exception(Exception::IllegalInstruction) => {
            // error!("[kernel] IllegalInstruction in application, core dumped.");
            error!(
                "[kernel] {:?} in application, bad addr = {:#x}, bad instruction = {:#x}, core dumped.",
                scause.cause(),
                current_trap_cx().sepc,
                stval,
            );
            // illegal instruction exit code
            exit_current_and_run_next(-3);
        }
        Trap::Interrupt(Interrupt::SupervisorTimer) => {            
            // debug!("SupervisorTimer");
        }
        Trap::Interrupt(Interrupt::SupervisorExternal) => {
            // debug!("Supervisor External");
            let current_task = current_task().unwrap();
            let mut inner = current_task.acquire_inner_lock();
            inner.interrupt_time += 1;
            drop(inner);
            plic::handle_external_interrupt(hart_id());
        }
        Trap::Interrupt(Interrupt::SupervisorSoft) => {
            // debug!("Supervisor Soft");
            unsafe { sip::clear_ssoft() }
        }
        _ => {
            error!(
                "Unsupported trap {:?}, stval = {:#x}!",
                scause.cause(),
                stval
            );
        }
    }
    trap_return();
}

#[no_mangle]
pub fn trap_return() -> ! {
    let task = current_task().unwrap();
    let mut task_inner = task.acquire_inner_lock();
    task_inner.last_user_time_us = get_time_us();
    drop(task_inner);
    drop(task);

    unsafe {
        sstatus::clear_sie();
    }
    set_user_trap_entry();
    let trap_cx_ptr = current_trap_cx_user_va();
    let user_satp = current_user_token();
    extern "C" {
        fn __alltraps();
        fn __restore();
    }
    // debug!("current pid: {}, tid: {}", current_process().unwrap().pid.0, sys_gettid());
    let restore_va = __restore as usize - __alltraps as usize + TRAMPOLINE;
    // trace!("return to user, trap frame: {:x?}", current_trap_cx());
    // push_trace(S_TRAP_RETURN + scause::read().bits());
    unsafe {
        sstatus::set_spie();
        sstatus::set_spp(sstatus::SPP::User);
        asm!("fence.i", "jr {}", in(reg) restore_va,
             in("a0") trap_cx_ptr, in("a1") user_satp)
    }
    panic!("Unreachable in back_to_user!");
}

#[no_mangle]
pub extern "C" fn trap_from_kernel(cx: &mut TrapContext) {
    debug!("trap_from_kernel");
    let scause = scause::read();
    let stval = stval::read();
    let sepc = sepc::read();
    let sstatus = sstatus::read();
    match scause.cause() {
        Trap::Interrupt(Interrupt::SupervisorSoft) => {
            debug!("SupervisorSoft");
        }
        _ => {
            error!(
                "Unsupported trap {:?}! stval = {:#x}, sepc = {:#x}, sstatus = {:#x?}, trap frame: {:x?}",
                scause.cause(),
                stval,
                sepc,
                sstatus,
                *cx
            );
            panic!("a trap {:?} from kernel!", scause::read().cause());
        }
    }
}
