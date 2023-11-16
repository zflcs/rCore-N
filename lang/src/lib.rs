//! This crate meets some basic requirements in in the riscv64-unknown-linux
//!

#![no_std]
#![no_main]
#![feature(lang_items, panic_info_message, format_args_nl)]
#![allow(internal_features)]

#[macro_use]
pub mod console;
extern crate alloc;

#[cfg(feature = "not_kernel")]
pub mod heap;

///
#[lang = "eh_personality"]
#[no_mangle]
pub extern "C" fn rust_eh_personality() {}

#[inline]
pub fn hart_id() -> usize {
    let hart_id: usize;
    unsafe {
        core::arch::asm!("mv {}, tp", out(reg) hart_id);
    }
    hart_id
}

#[cfg(feature = "kernel")]
pub mod lang_item {

    use sbi_rt::{system_reset, Shutdown,  SystemFailure};
    /// kernel panic
    #[panic_handler]
    fn panic(info: &core::panic::PanicInfo) -> ! {
        log::warn!("{info}");
        system_reset(Shutdown, SystemFailure);
        unreachable!()
    }
}

#[cfg(feature = "not_kernel")]
pub mod lang_item {
    /// kernel panic
    #[panic_handler]
    fn panic(info: &core::panic::PanicInfo) -> ! {
        log::warn!("{info}");
        unreachable!()
        // exit(-1);
    }
}

