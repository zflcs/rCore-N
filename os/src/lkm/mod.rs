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
pub use structs::ModuleSymbol;

pub fn init() {
    let spawn = LKM_MANAGER.lock().as_mut().unwrap().resolve_symbol("spawn").unwrap();
    vdso::init_spawn(spawn);
    let poll_kernel_future = LKM_MANAGER.lock().as_mut().unwrap().resolve_symbol("poll_kernel_future").unwrap();
    vdso::init_poll_kernel_future(poll_kernel_future);
    let re_back = LKM_MANAGER.lock().as_mut().unwrap().resolve_symbol("re_back").unwrap();
    vdso::init_re_back(re_back);
    let current_cid = LKM_MANAGER.lock().as_mut().unwrap().resolve_symbol("current_cid").unwrap();
    vdso::init_current_cid(current_cid);
    let max_prio = LKM_MANAGER.lock().as_mut().unwrap().resolve_symbol("max_prio").unwrap();
    vdso::init_max_prio(max_prio);
    let update_prio = LKM_MANAGER.lock().as_mut().unwrap().resolve_symbol("update_prio").unwrap();
    vdso::init_update_prio(update_prio);
}




















