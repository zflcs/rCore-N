#![no_std]
#![no_main]
#![feature(panic_info_message)]
#![feature(atomic_from_mut, inline_const)]
// #![deny(warnings, missing_docs)]


#[macro_use]
pub mod console;
#[macro_use]
pub mod kern_console;
pub mod config;

extern crate alloc;

use alloc::vec;

use alloc::boxed::Box;
use core::arch::asm;
use core::future::Future;
use core::pin::Pin;


#[panic_handler]
fn panic_handler(panic_info: &core::panic::PanicInfo) -> ! {
    let err = panic_info.message().unwrap();
    if let Some(location) = panic_info.location() {
        println!(
            "Panicked at {}:{}, {}",
            location.file(),
            location.line(),
            err
        );
    } else {
        println!("Panicked: {}", err);
    }
    exit(-1);
}

use core::alloc::{GlobalAlloc, Layout};
use crate::config::{HEAP_ORDER, HEAP_POINTER};
use buddy_system_allocator::LockedHeap;
type Heap = LockedHeap<HEAP_ORDER>;

/// 共享代码中默认的分配器，使用的是内核和用户程序各自的堆
/// 前提：堆的虚拟地址都保存在 HEAP_POINTER 这个虚拟地址中
/// 分配和回收时，先读取 HEAP_POINTER 虚拟地址中的内容
/// 再类型转换成正确的数据结构指针
/// 如果是把 heap 的指针当作参数传进需要使用的代码中，那么在分配的时候，需要显式的指出堆分配器
/// 通过这种方式，可以让默认的分配器使用不同的堆
#[global_allocator]
static GLOBAL: Global = Global;

struct Global;
unsafe impl GlobalAlloc for Global {
    #[inline]
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let heapptr = *(HEAP_POINTER as *const usize);
        let heap = heapptr as *mut usize as *mut Heap;
        (*heap).alloc(layout)
    }

    #[inline]
    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        let heapptr = *(HEAP_POINTER as *const usize);
        let heap = heapptr as *mut usize as *mut Heap;
        (*heap).dealloc(ptr, layout)
    }
}


use syscall::exit;

core::arch::global_asm!(include_str!("info.asm"));

/// _start() 函数，返回接口表的地址
#[no_mangle]
#[link_section = ".text.entry"]
extern "C" fn _start() -> usize {
    init_module()
}

// 自定义的模块接口，模块添加进地址空间之后，需要执行 _start() 函数填充这个接口表
static mut INTERFACE: [usize; 10] = [0; 10];

#[no_mangle]
#[inline(never)]
fn init_module() -> usize{
    unsafe {
        INTERFACE[0] = user_entry as usize;
        INTERFACE[1] = max_prio_pid as usize;
        INTERFACE[2] = spawn as usize;
        INTERFACE[3] = poll_kernel_future as usize;
        INTERFACE[4] = re_back as usize;
        INTERFACE[5] = current_cid as usize;
        INTERFACE[6] = reprio as usize;
        INTERFACE[7] = add_virtual_core as usize;
        INTERFACE[8] = update_prio as usize;
        INTERFACE[9] = get_pending_status as usize;
        &INTERFACE as *const [usize; 10] as usize
    }
}



use config::{ENTRY, MAX_THREAD_NUM, MAX_PROC_NUM, USER_HEAP_BASE, USER_HEAP_SIZE};
use core::sync::atomic::Ordering;
use core::sync::atomic::AtomicUsize;
use executor::{Executor, CoroutineId, CoroutineKind};

use syscall::*;
use core::task::Poll;


/// sret 进入用户态的入口，根据传递的堆指针，直接初始化堆
#[no_mangle]
#[inline(never)]
fn user_entry() {
    unsafe {
        let user_fn: fn() = core::mem::transmute(ENTRY);
        user_fn();
    }
    // 将主协程添加到 Executor 中
    let start = get_time();

    poll_user_future();
    wait_other_cores();

    let end = get_time();
    println!("total time: {} ms", end - start);
    
    exit(0);
}


/// 各个进程的最高优先级协程，通过共享内存的形式进行通信
pub static mut PRIO_ARRAY: [AtomicUsize; MAX_PROC_NUM + 1] = [const { AtomicUsize::new(usize::MAX) }; MAX_PROC_NUM + 1];

/// 进程的 Executor 调用这个函数，通过原子操作更新自己的最高优先级
#[no_mangle]
#[inline(never)]
pub fn update_prio(idx: usize, prio: usize) {
    unsafe {
        PRIO_ARRAY[idx].store(prio, Ordering::Relaxed);
    }
}

/// 内核重新调度进程时，调用这个函数，选出优先级最高的进程，再选出对应的线程
/// 所有进程的优先级相同时，则内核会优先执行协程，这里用 0 来表示内核的优先级
#[no_mangle]
#[inline(never)]
pub fn max_prio_pid() -> usize {
    let mut ret;
    let mut pid = 1;
    unsafe {
        ret = PRIO_ARRAY[1].load(Ordering::Relaxed);
    }
    for i in 1..MAX_PROC_NUM {
        unsafe {
            let prio = PRIO_ARRAY[i].load(Ordering::Relaxed);
            if prio < ret {
                ret = prio;
                pid = i;
            }
        }
    }
    pid
}


/// 添加协程，内核和用户态都可以调用
#[no_mangle]
#[inline(never)]
pub fn spawn(future: Pin<Box<dyn Future<Output=()> + 'static + Send + Sync>>, prio: usize, pid: usize, kind: CoroutineKind) -> usize {
    unsafe {
        let heapptr = *(HEAP_POINTER as *const usize);
        let exe = (heapptr + core::mem::size_of::<Heap>()) as *mut usize as *mut Executor;
        let cid = (*exe).spawn(future, prio, kind);
        // 更新优先级标记
        let prio = (*exe).priority;
        update_prio(pid, prio);
        // if pid == 0 {
        //     println_hart!("executor prio {}", hart_id(), prio);
        // } else {
        //     println!("executor prio {}", prio);
        // }
        return cid;
    }
}


/// 用户程序执行协程
#[no_mangle]
#[inline(never)]
pub fn poll_user_future() {
    unsafe {
        let heapptr = *(HEAP_POINTER as *const usize);
        let exe = (heapptr + core::mem::size_of::<Heap>()) as *mut usize as *mut Executor;
        let pid = getpid() as usize;
        let tid = gettid();
        loop {
            if (*exe).is_empty() {
                // println!("ex is empty");
                break;
            }
            let task = (*exe).fetch(tid as usize);
            match task {
                Some(task) => {
                    let cid = task.cid;
                    // println!("user task kind {:?}", task.kind);
                    match task.execute() {
                        Poll::Pending => {
                            (*exe).pending(cid.0);
                        }
                        Poll::Ready(()) => {
                            (*exe).del_coroutine(cid);
                        }
                    };
                    {
                        let _lock = (*exe).wr_lock.lock();
                        let prio: usize = (*exe).priority;
                        update_prio(getpid() as usize + 1, prio);
                    }
                }
                _ => {
                    // 任务队列不为空，但就绪队列为空，等待任务唤醒
                    yield_();
                }
            }
            // 执行完优先级最高的协程，检查优先级，判断是否让权
            let max_prio_pid = max_prio_pid();
            if pid + 1 != max_prio_pid {
                yield_();
            }
        }
        if tid != 0 {
            exit(2);
        }
    }
}
/// hart_id
#[allow(unused)]
pub fn hart_id() -> usize {
    let hart_id: usize;
    unsafe {
        core::arch::asm!("mv {}, tp", out(reg) hart_id);
    }
    hart_id
}
/// 内核执行协程
#[no_mangle]
#[inline(never)]
pub fn poll_kernel_future() {
    unsafe {
        let heapptr = *(HEAP_POINTER as *const usize);
        let exe = (heapptr + core::mem::size_of::<Heap>()) as *mut usize as *mut Executor;
        loop {
            let task = (*exe).fetch(hart_id());
            // 更新优先级标记
            let prio = (*exe).priority;
            update_prio(0, prio);
            match task {
                Some(task) => {
                    let cid = task.cid;
                    let kind = task.kind;
                    let _prio = task.inner.lock().prio;
                    match task.execute() {
                        Poll::Pending => {
                            (*exe).pending(cid.0);
                            if kind == CoroutineKind::KernSche {
                                // println_hart!("pending reback sche task{:?} kind {:?}", hart_id(), cid, kind);
                                re_back(cid.0, 0);
                            }
                        }
                        Poll::Ready(()) => {
                            (*exe).del_coroutine(cid);
                        }
                    };
                }
                _ => {
                }
            }
        }
    }
}
/// 获取当前正在执行的协程 id
#[no_mangle]
#[inline(never)]
pub fn current_cid(is_kernel: bool) -> usize {
    let tid = if is_kernel { hart_id() } else {
        gettid() as usize
    };
    assert!(tid < MAX_THREAD_NUM);
    unsafe {
        let heapptr = *(HEAP_POINTER as *const usize);
        let exe = (heapptr + core::mem::size_of::<Heap>()) as *mut usize as *mut Executor;
        (*exe).currents[tid].as_mut().unwrap().get_val()
    }
}

/// 协程重新入队，手动执行唤醒的过程，内核和用户都会调用这个函数
#[no_mangle]
#[inline(never)]
pub fn re_back(cid: usize, pid: usize) {
    // println!("[Exec]re back func enter");
    let mut start = 0;
    
    unsafe {
        let heapptr = *(HEAP_POINTER as *const usize);
        let exe = (heapptr + core::mem::size_of::<Heap>()) as *mut usize as *mut Executor;
        let prio = (*exe).re_back(CoroutineId(cid));
        // 重新入队之后，需要检查优先级
        let process_prio = PRIO_ARRAY[pid].load(Ordering::Relaxed);
        if prio < process_prio {
            PRIO_ARRAY[pid].store(prio, Ordering::Relaxed);
        }
    }
}

#[no_mangle]
#[inline(never)]
pub fn get_pending_status(cid: usize) -> bool {
    unsafe {
        let heapptr = *(HEAP_POINTER as *const usize);
        let exe = (heapptr + core::mem::size_of::<Heap>()) as *mut usize as *mut Executor;
        return (*exe).is_pending(cid)
    }
}

/// 更新协程优先级
#[no_mangle]
#[inline(never)]
pub fn reprio(cid: usize, prio: usize) {
    unsafe {
        let heapptr = *(HEAP_POINTER as *const usize);
        let exe = (heapptr + core::mem::size_of::<Heap>()) as *mut usize as *mut Executor;
        (*exe).reprio(CoroutineId(cid), prio);
    }
}

/// 申请虚拟CPU
#[no_mangle]
#[inline(never)]
pub fn add_virtual_core() {
    unsafe {
        let heapptr = *(HEAP_POINTER as *const usize);
        let exe = (heapptr + core::mem::size_of::<Heap>()) as *mut usize as *mut Executor;
        let tid = thread_create(poll_user_future as usize, 0) as usize;
        (*exe).add_wait_tid(tid);
    }
}


pub fn wait_other_cores() {
    unsafe {
        let heapptr = *(HEAP_POINTER as *const usize);
        let exe = (heapptr + core::mem::size_of::<Heap>()) as *mut usize as *mut Executor;
        for tid in (*exe).waits.iter() {
            waittid(*tid);
        }
    }
}
