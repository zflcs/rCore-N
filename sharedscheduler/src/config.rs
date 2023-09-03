/// 页面大小
pub const PAGE_SIZE: usize = 0x1000;

/// 256MB physical memory
pub const PHYSICAL_MEMORY_END: usize = 0x9000_0000;

/// heap pointer
pub const HEAP_POINTER: usize = PHYSICAL_MEMORY_END + PAGE_SIZE;

/// Used for buddy system allocator
pub const HEAP_ORDER: usize = 32;

/// 用户程序入口
pub const ENTRY: usize = 0x1000;
/// CPU数量 + 用户态中断处理线程
pub const MAX_THREAD: usize = 30;

/// 协程支持的优先级数目
pub const PRIO_NUM: usize = 8;

/// User heap base
pub const USER_HEAP_BASE: usize = 0x4000_0000;

/// User heap size
pub const USER_HEAP_SIZE: usize = 0x40_0000;

/// global bitmap virtual address
pub const GLOBAL_BITMAP_BASE: usize = HEAP_POINTER + PAGE_SIZE;



