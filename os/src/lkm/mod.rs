pub mod const_reloc;
pub mod manager;
pub mod structs;
pub mod api;
pub use manager::LKM_MANAGER;

use core::future::Future;
use alloc::boxed::Box;

pub fn spawn(fut: Box<dyn Future<Output = i32> + 'static + Send + Sync>, priority: u32) {
    let spawn_ptr = LKM_MANAGER.lock().resolve_symbol("spawn").unwrap();
    unsafe {
        let spawn_fn: fn(Box<dyn Future<Output = i32> + 'static + Send + Sync>, u32) = core::mem::transmute(spawn_ptr);
        spawn_fn(fut, priority);
    }
}

pub fn entry() {
    let entry_ptr = LKM_MANAGER.lock().resolve_symbol("entry").unwrap();
    unsafe {
        let entry_fn: fn() = core::mem::transmute(entry_ptr);
        entry_fn();
    }
}
