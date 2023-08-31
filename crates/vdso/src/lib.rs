
#![no_std]

extern crate alloc;


mod symbol;
pub use symbol::*;

use vdso_macro::get_libfn;
use executor::CoroutineKind;
use alloc::boxed::Box;
use core::future::Future;
use core::pin::Pin;

// get_libfn!(
//     pub fn spawn(f: Pin<Box<dyn Future<Output=()> + 'static + Send + Sync>>, prio: usize, pid: usize, kind: CoroutineKind) -> usize {}
// );
/// spawn 单独列出是因为在过程宏中，没有实现复杂的参数解析
#[no_mangle]
#[link_section = ".vdso.spawn"]
pub static mut VDSO_SPAWN: usize = 0;
#[cfg(feature = "kernel")]
pub fn init_spawn(ptr:usize){
  unsafe {
    VDSO_SPAWN = ptr;
  }
}
#[inline(never)]
pub fn spawn<F, T>(f: F, prio: usize, pid: usize, kind: CoroutineKind) -> usize 
where 
    F: FnOnce() -> T,
    T: Future<Output = ()> + 'static + Send + Sync
{
  unsafe {
    let func:fn(f:Pin<Box<dyn Future<Output = ()> +'static+Send+Sync> > ,prio:usize,pid:usize,kind:CoroutineKind) -> usize = core::mem::transmute(VDSO_SPAWN);
    func(Box::pin(f()),prio,pid,kind)
  }
}

get_libfn!(
    pub fn current_cid(is_kernel: bool) -> usize {}
);

get_libfn!(
    pub fn re_back(cid: usize, pid: usize) {}
);

get_libfn!(
    pub fn add_virtual_core() {}
);

get_libfn!(
  pub fn max_prio() -> Option<usize> {}
);

get_libfn!(
    pub fn poll_kernel_future() {}
);

get_libfn!(
    pub fn reprio(cid: usize, prio: usize) {}
);

get_libfn!(
  fn update_prio(prio: usize, is_add: bool) {}
);

get_libfn!(
  pub fn get_pending_status(cid: usize) -> bool {}
);