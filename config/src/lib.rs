#![no_std]

/// CPU 数目
pub const CPU_NUM: usize = 4;
/// trace 大小
pub const TRACE_SIZE: usize = 0x1000_0000; // 256M

/// 页面大小
pub const PAGE_SIZE: usize = 0x1000;
pub const PAGE_SIZE_BITS: usize = 0xc;

/// 堆栈大小
pub const USER_STACK_SIZE: usize = 0x4000;
pub const KERNEL_STACK_SIZE: usize = 0x4000;
pub const KERNEL_HEAP_SIZE: usize = 0x200_0000;
pub const USER_HEAP_SIZE: usize = 1 << 20;


/// 跳板页虚拟地址
pub const TRAMPOLINE: usize = usize::MAX - PAGE_SIZE + 1;
/// 用户态中断虚拟地址
pub const USER_TRAP_BUFFER: usize = TRAMPOLINE - PAGE_SIZE;
/// 共享调度器使用的数据所在的虚拟地址，在这个位置记录了用户程序堆的虚拟地址
/// 在共享代码中操作不同进程的堆和 Executor 主要是读取这个虚拟地址中保存的用户程序堆 heap 的虚拟地址
/// 再来进行分配
pub const HEAP_BUFFER: usize = USER_TRAP_BUFFER - PAGE_SIZE;
/// 陷入上下文虚拟地址
pub const TRAP_CONTEXT: usize = HEAP_BUFFER - PAGE_SIZE;

pub const VMM_MANAGER_TOP: usize = 0xFFFFFFFF00000000;

/// 用户程序相关设置
/// 进程入口
pub const ENTRY: usize = 0x1000;
/// CPU数量 + 用户态中断处理线程
pub const MAX_THREAD_NUM: usize = 30;
/// 协程支持的优先级数目
pub const PRIO_NUM: usize = 8;
/// 单个优先级下支持的最大协程数
pub const PER_PRIO_COROU: usize = 8192;
/// 支持的最大进程数量
pub const MAX_PROC_NUM: usize = 0x1000;
/// Executor 记录的优先级地址
pub const PRIO_PTR: usize = HEAP_BUFFER + 0x20;

/// 内核结束位置
#[cfg(feature = "board_qemu")]
pub const MEMORY_END: usize = 0x84000000;

#[cfg(feature = "board_lrv")]
// pub const MEMORY_END: usize = 0x100A00000;
pub const MEMORY_END: usize = 0x10600_0000;

/// 时钟频率
#[cfg(feature = "board_qemu")]
pub const CLOCK_FREQ: usize = 12500000;

#[cfg(feature = "board_lrv")]
pub const CLOCK_FREQ: usize = 10_000_000;







