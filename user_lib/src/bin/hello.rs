#![no_std]
#![no_main]

#[macro_use]
extern crate lang;
#[macro_use]
extern crate syscall;



#[no_mangle]
#[link_section = ".text.entry"]
pub extern "C" fn __libc_start_main() {
    main();
}

#[no_mangle]
fn main() -> i32 {
    0
}