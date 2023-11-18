use crate::config::KERNEL_HEAP_SIZE;
use alloc::{collections::VecDeque, vec};
use buddy_system_allocator::LockedHeap;
use core::{
    alloc::{GlobalAlloc, Layout},
    ptr::NonNull,
};
use executor::Executor;
use spin::{Lazy, Mutex};

#[alloc_error_handler]
pub fn handle_alloc_error(layout: core::alloc::Layout) -> ! {
    panic!("Heap allocation error, layout = {:?}", layout);
}

#[no_mangle]
#[link_section = ".data.heap"]
pub static mut HEAP: LockedHeap<32> = LockedHeap::new();


#[no_mangle]
#[link_section = ".data.executor"]
pub static mut EXECUTOR: Lazy<Executor> = Lazy::new(|| Executor::new());

#[no_mangle]
#[link_section = ".bss.memory"]
static mut MEMORY: [u8; KERNEL_HEAP_SIZE] = [0u8; KERNEL_HEAP_SIZE];

/// 初始化全局分配器和内核堆分配器。
pub fn init_heap() {
    unsafe {
        HEAP.lock().init(MEMORY.as_ptr() as usize, KERNEL_HEAP_SIZE);
    }
}

struct Global;

#[global_allocator]
static GLOBAL: Global = Global;

unsafe impl GlobalAlloc for Global {
    #[inline]
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        HEAP.lock()
            .alloc(layout)
            .ok()
            .map_or(0 as *mut u8, |allocation| allocation.as_ptr())
    }

    #[inline]
    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        HEAP.lock().dealloc(NonNull::new_unchecked(ptr), layout)
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


