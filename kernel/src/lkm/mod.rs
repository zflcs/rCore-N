

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
    let re_back = LKM_MANAGER.lock().as_mut().unwrap().resolve_symbol("re_back").unwrap();
    vdso::init_re_back(re_back);
    let current_cid = LKM_MANAGER.lock().as_mut().unwrap().resolve_symbol("current_cid").unwrap();
    vdso::init_current_cid(current_cid);
}




















