use crate::loader::get_app_data_by_name;
use alloc::vec::Vec;
use vdso::get_dynsym_addr;
use crate::mm::{KERNEL_SPACE, MemorySet};
use lazy_static::*;
use alloc::sync::Arc;
use core::mem::transmute;
use alloc::boxed::Box;
use xmas_elf::ElfFile;

pub mod manager;
pub use manager::*;
mod structs;
pub mod api;
pub use api::*;
mod const_reloc;




















