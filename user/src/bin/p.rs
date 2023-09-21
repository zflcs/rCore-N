#![no_std]
#![no_main]

use user_syscall::sleep_ms;


extern crate user_lib;
extern crate alloc;

static mut NUM: usize = 0;

#[no_mangle]
pub fn main() -> i32 {
    // loop {
    //     sleep_ms(10);
    // };
    0
}

