//! This crate meets some basic requirements in in the riscv64-unknown-linux
//! 


#![no_std]
#![no_main]
#![feature(lang_items)]
#![allow(internal_features, unused)]

use core::pin::Pin;
use core::alloc::Layout;

mod heap;


/// kernel panic
#[panic_handler]
fn panic(info: &core::panic::PanicInfo) -> ! {
    unreachable!()
}

///
#[lang = "eh_personality"]
#[no_mangle]
pub extern fn rust_eh_personality() {}


