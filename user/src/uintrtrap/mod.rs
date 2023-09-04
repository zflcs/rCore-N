mod trap;

use riscv::register::uip;
use uintr::*;

#[repr(C)]
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
}

#[no_mangle]
pub extern "C" fn handler_entry(uintr_frame: &mut UintrFrame, handler_ptr: usize) {
    unsafe {
        let mut irqs = uipi_read();
        uip::clear_usoft();
        let handler: fn(&mut UintrFrame, usize) -> usize = core::mem::transmute(handler_ptr);
        irqs = handler(uintr_frame, irqs);
        uipi_write(irqs);
    }
}

pub fn uintr_register_receier(handler_ptr: usize) -> usize {
    extern "C" {
        fn uintrvec();
    }
    unsafe { 
        utvec::write(uintrvec as usize, utvec::TrapMode::Direct);
        uscratch::write(handler_ptr);
        ustatus::set_uie();
        uie::set_usoft();
    }
    // println!("utvec {:#x?}", uintrvec as usize);
    // println!("uscratch {:#x?}", handler_ptr);
    user_syscall::uintr_register_receiver()
}