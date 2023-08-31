use buddy_system_allocator::LockedHeap;
use log::error;
use executor::Executor;

use super::config::{KERNEL_HEAP_ORDER, KERNEL_HEAP_SIZE};

#[global_allocator]
#[no_mangle]
#[link_section = ".data.heap"]
static HEAP_ALLOCATOR: LockedHeap<KERNEL_HEAP_ORDER> = LockedHeap::<KERNEL_HEAP_ORDER>::empty();


#[no_mangle]
#[link_section = ".data.executor"]
pub static mut EXECUTOR: Executor = Executor::new(true);


#[alloc_error_handler]
pub fn handle_alloc_error(layout: core::alloc::Layout) -> ! {
    error!("[kernel] Heap allocation error: {:x?}", layout);
    panic!()
}

static mut HEAP_SPACE: [u8; KERNEL_HEAP_SIZE] = [0; KERNEL_HEAP_SIZE];

pub fn init() {
    unsafe {
        HEAP_ALLOCATOR
            .lock()
            .init(HEAP_SPACE.as_ptr() as usize, KERNEL_HEAP_SIZE);
    }
}
