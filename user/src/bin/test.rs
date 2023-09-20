#![no_std]
#![no_main]

#[macro_use]
extern crate user_lib;
extern crate alloc;

use user_lib::{UintrFrame, uintr_register_receier};
use user_syscall::{exit, uintr_create_fd, close, uintr_test};

static mut UINTR_RECEIVED: bool = false;
static mut UINTR_FD: isize = 0;

#[no_mangle]
pub fn main() -> i32 {
    println!("Basic test: kernel uintr test");
    if uintr_register_receier(uintr_handler as usize) != 0 {
        println!("Interrupt handler register error");
        exit(-1);
    }
    unsafe { UINTR_FD = uintr_create_fd(1) };
    if unsafe { UINTR_FD } < 0 {
        println!("Interrupt vector allocation error");
        exit(-2);
    }
    println!("Receiver enabled interrupts");
    uintr_test(unsafe { UINTR_FD as usize });

    let ptr = unsafe { &mut UINTR_RECEIVED as *mut bool };
    while unsafe { !ptr.read_volatile() } { }
    close(unsafe { UINTR_FD as usize });
    println!("success");
    0
}

#[no_mangle]
pub extern "C" fn uintr_handler(_uintr_frame: &mut UintrFrame, irqs: usize) -> usize {
    println!("\t-- User Interrupt handler --");
    // read pending bits
    println!("\tPending User Interrupts: {:b}", irqs);
    unsafe { UINTR_RECEIVED = true };
    println!("UINTR_RECEIVED {}", unsafe { UINTR_RECEIVED } );
    use executor::{MessageQueue, MESSAGE_QUEUE_ADDR};
    let queue = unsafe { &mut *(MESSAGE_QUEUE_ADDR as *mut MessageQueue) };
    while let Some(message) = queue.dequeue() {
        println!("message {:?}", message);
    }

    return 0;
}

