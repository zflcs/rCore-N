//! This crate defines the structures related to `async` runtime.
//! 
#![cfg_attr(not(test), no_std)]
#![feature(sync_unsafe_cell)]
#![feature(associated_type_bounds)]
#![warn(missing_docs)]


extern crate alloc;

mod queue;
mod task;
mod waker;
mod executor;


pub use task::*;
pub use executor::*;

pub use priority::PRIO_LEVEL;

/// 
mod priority {
    #[cfg(feature = "prio-level-4")]
    ///
    pub const PRIO_LEVEL: usize = 4;
    #[cfg(feature = "prio-level-8")]
    ///
    pub const PRIO_LEVEL: usize = 8;
    #[cfg(feature = "prio-level-16")]
    ///
    pub const PRIO_LEVEL: usize = 16;
}
