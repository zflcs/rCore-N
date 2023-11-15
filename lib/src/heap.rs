

use core::{ptr::NonNull, alloc::{GlobalAlloc, Layout}};

use buddy_system_allocator::LockedHeap;
use config::HEAP_LOCATION;

/// 
#[global_allocator]
static GLOBAL: Global = Global;

struct Global;
unsafe impl GlobalAlloc for Global {
    #[inline]
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let heapptr = *(HEAP_LOCATION as *const usize);
        let heap = heapptr as *mut usize as *mut LockedHeap<32>;
        (*heap).lock().alloc(layout).ok()
        .map_or(0 as *mut u8, |allocation| allocation.as_ptr())
    }

    #[inline]
    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        let heapptr = *(HEAP_LOCATION as *const usize);
        let heap = heapptr as *mut usize as *mut LockedHeap<32>;
        (*heap).lock().dealloc(NonNull::new_unchecked(ptr), layout)
    }
}
