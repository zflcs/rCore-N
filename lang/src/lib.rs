//! This crate meets some basic requirements in in the riscv64-unknown-linux
//!

#![no_std]
#![no_main]
#![feature(lang_items, panic_info_message, alloc_error_handler)]
#![allow(internal_features, non_snake_case)]

#[macro_use]
pub mod console;
extern crate alloc;

#[cfg(feature = "not_kernel")]
pub mod heap;


#[inline]
pub fn hart_id() -> usize {
    let hart_id: usize;
    unsafe {
        core::arch::asm!("mv {}, tp", out(reg) hart_id);
    }
    hart_id
}

#[cfg(feature = "kernel")]
pub mod kernel_lang_item {

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
    ///
    #[lang = "eh_personality"]
    #[no_mangle]
    pub fn rust_eh_personality() {}

    #[no_mangle]
    pub fn memcpy() {}

    #[no_mangle]
    pub fn __cxa_finalize() {}

    #[no_mangle]
    pub fn _Unwind_Resume() {}

    #[no_mangle]
    pub fn _ITM_registerTMCloneTable() {}

    #[no_mangle]
    pub fn _ITM_deregisterTMCloneTable() {}

    #[no_mangle]
    pub fn memset() {}

    /// not_kernel panic
    #[panic_handler]
    fn panic(info: &core::panic::PanicInfo) -> ! {
        log::warn!("{info}");
        syscall::exit(-1)
    }
}

