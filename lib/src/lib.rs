//! This crate meets some basic requirements in in the riscv64-unknown-linux
//!

#![no_std]
#![no_main]
#![feature(lang_items)]
#![allow(internal_features)]

pub mod console;
pub mod heap;

#[cfg(feature = "lang")]
pub mod lang_item {
    /// kernel panic
    #[panic_handler]
    fn panic(_info: &core::panic::PanicInfo) -> ! {
        unreachable!()
    }

    ///
    #[lang = "eh_personality"]
    #[no_mangle]
    pub extern "C" fn rust_eh_personality() {}
}
