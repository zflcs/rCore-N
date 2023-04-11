#![no_std]
#![feature(lang_items)]
#![feature(panic_info_message)]
#![feature(alloc_error_handler)]
// #![feature(allocator_api)]

// #![deny(warnings, missing_docs)]

#[cfg(feature = "inner")]
#[macro_use]
pub mod console;

#[cfg(feature = "inner")]
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







