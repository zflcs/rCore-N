#![no_std]
#![no_main]

#[macro_use]
extern crate user_lib;
extern crate alloc;
use alloc::vec::Vec;

#[no_mangle]
pub fn main(argc: usize, argv: Vec<&'static str>) -> i32 {
    println!("[hello world] arg_len {} args: {:?}", argc, argv);
    0
}

