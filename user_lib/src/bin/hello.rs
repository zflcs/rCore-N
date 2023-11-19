#![no_std]
#![no_main]

#[macro_use]
extern crate lang;
extern crate syscall;
extern crate alloc;
use alloc::boxed::Box;
use core::future::Future;
#[no_mangle]
pub fn main() -> Box<dyn Future<Output = i32> + 'static + Send + Sync> {
    init_heap();
    init_executor();
    let mut a = alloc::vec::Vec::new();
    a.push(56);
    println!("ok {}", a.pop().unwrap());
    Box::new(test_fn())
}


async fn test_fn() -> i32 {
    println!("into user test");
    0
}

#[no_mangle]
pub extern "C" fn put_str(ptr: *const u8, len: usize) {
    syscall::sys_write(1, ptr as _, len, usize::MAX, usize::MAX);
}


use buddy_system_allocator::LockedHeap;
use core::{
    alloc::Layout,
    ptr::NonNull,
};
use executor::Executor;
use spin::Once;


#[no_mangle]
#[link_section = ".data.heap"]
pub static mut HEAP: LockedHeap<32> = LockedHeap::new();


#[no_mangle]
#[link_section = ".data.executor"]
pub static mut EXECUTOR: Once<Executor> = Once::new();

pub const USER_HEAP_SIZE: usize = 0x40000;

#[no_mangle]
#[link_section = ".bss.memory"]
static mut MEMORY: [u8; USER_HEAP_SIZE] = [0u8; USER_HEAP_SIZE];

/// 
fn init_heap() {
    unsafe {
        HEAP.lock().init(MEMORY.as_ptr() as usize, USER_HEAP_SIZE);
    }
}

/// init
fn init_executor() {
    unsafe {
        EXECUTOR.call_once(|| Executor::new());
    }
}

#[no_mangle]
pub unsafe extern "C" fn alloc(size: usize, align: usize) -> *mut u8 {
    HEAP.lock()
        .alloc(Layout::from_size_align_unchecked(size, align))
        .ok()
        .map_or(0 as *mut u8, |allocation| allocation.as_ptr())
}

#[no_mangle]
pub unsafe extern "C" fn dealloc(ptr: *mut u8, size: usize, align: usize) {
    HEAP.lock().dealloc(
        NonNull::new_unchecked(ptr), 
        Layout::from_size_align_unchecked(size, align)
    )
}



