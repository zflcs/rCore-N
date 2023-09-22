#![no_std]
#![no_main]

extern crate alloc;

extern crate user_lib;

use user_lib::*;

static mut UINTR_RECEIVED: bool = false;

#[no_mangle]
pub fn main() -> i32 {
    let init_res = init_uintr_trap(uintr_handler as usize);
    println!("Enabled user interrupts, trap_info_base {:#x}", init_res);
    sys_uintr_test();
    let ptr = unsafe { &mut UINTR_RECEIVED as *mut bool };
    while unsafe { !ptr.read_volatile() } { 
        println!("main UINTR_RECEIVED {}", unsafe { UINTR_RECEIVED } );
    }
    println!("main UINTR_RECEIVED {}", unsafe { UINTR_RECEIVED } );
    0
}


#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct UserTrapRecord {
    pub cause: usize,
    pub message: usize,
}
use heapless::spsc::Queue;
pub type UserTrapQueue = Queue<UserTrapRecord, 128>;

/// Do not use system calls or other time-consuming operations 
/// otherwise, there will be something error
#[no_mangle]
pub extern "C" fn uintr_handler(_uintr_frame: &mut UintrFrame) -> usize {
    let trap_queue = unsafe { &mut *(0xFFFFFFFFFFFFE000 as *mut UserTrapQueue) };
    while let Some(trap_record) = trap_queue.dequeue() { }
    unsafe { UINTR_RECEIVED = true };
    return 0;
}

