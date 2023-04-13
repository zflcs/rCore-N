#![no_std]
#![feature(naked_functions)]
#![feature(panic_info_message)]
#![feature(allocator_api)]
#![feature(atomic_from_mut, inline_const)]
#![feature(linkage)]
#![feature(alloc_error_handler)]
#![feature(lang_items)]
#![no_builtins]

// #![deny(warnings, missing_docs)]

#[macro_use]
pub mod console;

#[macro_use]
pub mod kern_console;

extern crate alloc;

pub use config::*;

#[cfg(feature = "inner")]
mod lang_items;
#[cfg(feature = "inner")]
pub use lang_items::*;

mod runtime;
pub use runtime::*;







