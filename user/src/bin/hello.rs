#![no_std]
#![no_main]

#[macro_use]
extern crate user_lib;
extern crate alloc;

use uintr::uipi_send;
use user_lib::{UintrFrame, uintr_register_receier};
use user_syscall::{exit, uintr_create_fd, fork, uintr_register_sender, close};

static mut UINTR_RECEIVED: bool = false;
static mut UINTR_FD: isize = 0;

#[no_mangle]
pub fn main() -> i32 {
    println!("Basic test: uipi_sample");
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

    // thread_create(sender_thread as usize);
    let pid = fork();
    if pid == 0 {
        let uipi_index = uintr_register_sender(unsafe { UINTR_FD as usize });
        if uipi_index < 0 {
            println!("Sending IPI from sender thread");
            exit(-3);
        }
        println!("Sending IPI from sender thread {}", uipi_index);
        unsafe { uipi_send(uipi_index as usize) };
        exit(0);
    } else {
        let ptr = unsafe { &mut UINTR_RECEIVED as *mut bool };
        while unsafe { !ptr.read_volatile() } { }
        close(unsafe { UINTR_FD as usize });
        println!("success");
    }
    0
}

#[no_mangle]
pub extern "C" fn uintr_handler(_uintr_frame: &mut UintrFrame, irqs: usize) -> usize {
    println!("\t-- User Interrupt handler --");
    // read pending bits
    println!("\tPending User Interrupts: {:b}", irqs);
    unsafe { UINTR_RECEIVED = true };
    println!("UINTR_RECEIVED {}", unsafe { UINTR_RECEIVED } );
    return 0;
}

// pub fn sender_thread() {
//     let uipi_index = uintr_register_sender(unsafe { UINTR_FD as usize });
//     if uipi_index < 0 {
//         println!("Sending IPI from sender thread");
//         exit(-3);
//     }
//     println!("Sending IPI from sender thread {}", uipi_index);
//     unsafe { uipi_send(uipi_index as usize) };
// }

// pub fn thread_create(func: usize) {
//     let pid = fork(17 | 256);
//     if pid == 0 {
//         let thread: fn() = unsafe { core::mem::transmute(func) };
//         thread();
//         exit(0);
//     } else {
//         while unsafe { !UINTR_RECEIVED } {
//             println!("loop");
//         }
//         close(unsafe { UINTR_FD as usize });
//         println!("success");
//     }
// }
