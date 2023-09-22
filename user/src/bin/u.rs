#![no_std]
#![no_main]

extern crate alloc;

extern crate user_lib;

use riscv::register::ustatus;
use user_lib::*;

static mut UINTR_RECEIVED: bool = false;

#[no_mangle]
pub fn main() -> i32 {
    let init_res = init_uintr_trap();
    println!("Enabled user interrupts, trap_info_base {:#x}", init_res);
    sys_uintr_test();
    let ptr = unsafe { &mut UINTR_RECEIVED as *mut bool };
    while unsafe { !ptr.read_volatile() } { 
        println!("main UINTR_RECEIVED {}", unsafe { UINTR_RECEIVED } );
    }
    println!("main UINTR_RECEIVED {}", unsafe { UINTR_RECEIVED } );
    0
}


#[no_mangle]
pub fn wake_handler(cid: usize) {
    unsafe { UINTR_RECEIVED = true };
}
