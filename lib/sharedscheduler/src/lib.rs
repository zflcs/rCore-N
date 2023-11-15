//! This crate provides the runtime of sharedscheduler
//! 

#![no_std]
#![no_main]

#[macro_use]
extern crate lib;
extern crate alloc;

use alloc::vec;

#[no_mangle]
pub fn test() -> i32 {
    let mut _a = vec![1, 3, 4];
    _a.push(2);
    println!("test");
    2
}