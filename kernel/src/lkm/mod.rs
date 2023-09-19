

pub mod manager;
pub use manager::*;
mod structs;
pub mod api;
pub use api::*;
mod const_reloc;
pub use structs::ModuleSymbol;


pub fn init() {
    ModuleManager::init();
    let _ = LKM_MANAGER.lock().as_mut().unwrap().init_module("sharedscheduler", "");
    let spawn = LKM_MANAGER.lock().as_mut().unwrap().resolve_symbol("spawn").unwrap();
    vdso::init_spawn(spawn);
    let wake = LKM_MANAGER.lock().as_mut().unwrap().resolve_symbol("wake").unwrap();
    vdso::init_wake(wake);
    let current_cid = LKM_MANAGER.lock().as_mut().unwrap().resolve_symbol("current_cid").unwrap();
    vdso::init_current_cid(current_cid);
}




















