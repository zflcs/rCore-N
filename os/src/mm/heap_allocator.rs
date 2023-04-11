
use alloc::{vec, collections::VecDeque};
use core::{
    alloc::{GlobalAlloc, Layout},
    ptr::NonNull,
};
use basic::Executor;
use spin::Mutex;
use config::KERNEL_HEAP_SIZE;
use buddy_system_allocator::LockedHeap;

#[alloc_error_handler]
pub fn handle_alloc_error(layout: core::alloc::Layout) -> ! {
    panic!("Heap allocation error, layout = {:?}", layout);
}

#[no_mangle]
#[link_section = ".data.heap"]
pub static mut HEAP: LockedHeap = LockedHeap::empty();

#[no_mangle]
#[link_section = ".data.executor"]
pub static mut EXECUTOR: Executor = Executor::new();

#[no_mangle]
#[link_section = ".bss.memory"]
static mut MEMORY: [u8; KERNEL_HEAP_SIZE] = [0u8; KERNEL_HEAP_SIZE];

use config::PER_PRIO_COROU;
use basic::CoroutineId;
use heapless::mpmc::MpMcQueue;
pub type FreeLockQueue = MpMcQueue<CoroutineId, PER_PRIO_COROU>;
const QUEUE_CONST: FreeLockQueue = FreeLockQueue::new();
/// 初始化全局分配器和内核堆分配器。
pub fn init_heap() {

    unsafe {
        HEAP.lock().init(
            MEMORY.as_ptr() as usize,
            KERNEL_HEAP_SIZE,
        );        
    }
    // error!("heap {:#x}", unsafe{ &mut HEAP as *mut LockedHeap as usize });
    // error!("EXECUTOR ptr {:#x}", unsafe{ &mut EXECUTOR as *mut Executor as usize });
    unsafe {
        EXECUTOR.ready_queue = [QUEUE_CONST; config::PRIO_NUM];
    }
}


struct Global;

#[global_allocator]
static GLOBAL: Global = Global;

unsafe impl GlobalAlloc for Global {
    #[inline]
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        HEAP.alloc(layout)
    }

    #[inline]
    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        HEAP.dealloc(ptr, layout)
    }
}


