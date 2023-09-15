use core::alloc::{GlobalAlloc, Layout};
use executor::{Executor, EMPTY_QUEUE, MAX_PRIO};
use buddy_system_allocator::LockedHeap;
const HEAP_ORDER: usize = 32;
type Heap = LockedHeap<HEAP_ORDER>;

#[no_mangle]
#[link_section = ".data.heap"]
pub static mut HEAP: Heap = Heap::empty();

#[no_mangle]
#[link_section = ".data.executor"]
pub static mut EXECUTOR: Executor = Executor::new();

// 托管空间 16 KiB
const MEMORY_SIZE: usize = 2 << 21;
#[no_mangle]
#[link_section = ".data.memory"]
static mut MEMORY: [u8; MEMORY_SIZE] = [0u8; MEMORY_SIZE];


/// 初始化全局分配器和内核堆分配器。
pub fn init() {
    unsafe {
        HEAP.lock().init(
            MEMORY.as_ptr() as usize,
            MEMORY_SIZE,
        );
        EXECUTOR.ready_queue = [EMPTY_QUEUE; MAX_PRIO];
        // EXECUTOR.ready_queue = vec![VecDeque::new(); executor::PRIO_NUM];
        // println!("heap {:#x}", &mut HEAP as *mut Heap as usize);
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





