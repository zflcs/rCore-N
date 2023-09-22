mod trap;

use riscv::register::{uip, utvec, uscratch, ustatus, uie, ucause, utval};
use syscall::sys_uintr_init;

#[repr(C)]
#[derive(Debug)]
pub struct UintrFrame {
    pub ra: usize,
    pub sp: usize,
    pub gp: usize,
    pub tp: usize,
    pub t0: usize,
    pub t1: usize,
    pub t2: usize,
    pub s0: usize,
    pub s1: usize,
    pub a0: usize,
    pub a1: usize,
    pub a2: usize,
    pub a3: usize,
    pub a4: usize,
    pub a5: usize,
    pub a6: usize,
    pub a7: usize,
    pub s2: usize,
    pub s3: usize,
    pub s4: usize,
    pub s5: usize,
    pub s6: usize,
    pub s7: usize,
    pub s8: usize,
    pub s9: usize,
    pub s10: usize,
    pub s11: usize,
    pub t3: usize,
    pub t4: usize,
    pub t5: usize,
    pub t6: usize,
    pub uepc: usize,
}

#[no_mangle]
pub extern "C" fn handler_entry(uintr_frame: &mut UintrFrame, handler_ptr: usize) {
    unsafe {
        uip::clear_usoft();
        let ucause = ucause::read();
        let utval = utval::read();
        println!("{:?} {:#x?}", ucause.cause(), utval);
        let handler: fn(&mut UintrFrame) -> usize = core::mem::transmute(handler_ptr);
        let res = handler(uintr_frame);
    }
}

// init uintr_trap and alloc trap_info record
pub fn init_uintr_trap() -> isize {
    extern "C" {
        fn __alltraps_u();
    }
    unsafe {
        utvec::write(__alltraps_u as usize, utvec::TrapMode::Direct);
        ustatus::set_uie();
        uie::set_usoft();
    }
    // set uintr trap handler tid
    let ans = sys_uintr_init(0);
    ans
}