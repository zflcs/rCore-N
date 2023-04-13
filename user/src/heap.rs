use alloc::{vec, collections::VecDeque};
use core::{
    alloc::{GlobalAlloc, Layout},
};
use basic::{Executor, CoroutineId};
use buddy_system_allocator::LockedHeap;
use config::PER_PRIO_COROU;
use heapless::mpmc::MpMcQueue;
type TaskQueue = MpMcQueue<CoroutineId, PER_PRIO_COROU>;
const EMPTY_QUEUE: TaskQueue = TaskQueue::new();

#[no_mangle]
#[link_section = ".data.heap"]
pub static mut HEAP: LockedHeap = LockedHeap::empty();

#[no_mangle]
#[link_section = ".data.executor"]
pub static mut EXECUTOR: Executor = Executor::new();

// 托管空间 16 KiB
const MEMORY_SIZE: usize = 1 << 20;
#[no_mangle]
#[link_section = ".data.memory"]
static mut MEMORY: [u8; MEMORY_SIZE] = [0u8; MEMORY_SIZE];


/// 初始化全局分配器和内核堆分配器。
pub fn init() {
    // println!("EXECUTOR ptr {:#x}", unsafe{ &mut EXECUTOR as *mut Executor as usize });

    unsafe {
        HEAP.lock().init(
            MEMORY.as_ptr() as usize,
            MEMORY_SIZE,
        );
        // HEAP.lock().transfer(NonNull::new_unchecked(MEMORY.as_mut_ptr()), MEMORY.len());
    }
    unsafe {
        EXECUTOR.ready_queue = [EMPTY_QUEUE; config::PRIO_NUM];
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

