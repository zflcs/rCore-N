//! Executor 运行时
#![no_std]
#![no_main]
#![feature(inline_const)]

#[macro_use]
extern crate basic;
extern crate alloc;

use config::{ENTRY, MAX_THREAD_NUM, HEAP_BUFFER, PRIO_NUM};
use basic::{CoroutineId, Executor, CoroutineKind};
use alloc::boxed::Box;
use core::pin::Pin;
use core::future::Future;
use core::task::Poll;
use syscall::*;
use core::sync::atomic::Ordering;
use core::sync::atomic::AtomicUsize;
use buddy_system_allocator::LockedHeap;

core::arch::global_asm!(include_str!("info.asm"));


// 为了不让没有使用的函数被编译器优化
static mut INTERFACE: [usize; 10] = [0; 10];

#[no_mangle]
fn main() -> usize{
    init_module()
}

#[no_mangle]
#[inline(never)]
pub fn init_module() -> usize {
    unsafe {
        INTERFACE[0] = user_entry as usize;
        INTERFACE[1] = max_prio as usize;
        INTERFACE[2] = spawn as usize;
        INTERFACE[3] = poll_kernel_future as usize;
        INTERFACE[4] = re_back as usize;
        INTERFACE[5] = current_cid as usize;
        INTERFACE[6] = reprio as usize;
        INTERFACE[7] = add_virtual_core as usize;
        INTERFACE[8] = update_prio as usize;
        &INTERFACE as *const [usize; 10] as usize
    }
}

/// sret 进入用户态的入口，在这个函数再执行 main 函数
#[no_mangle]
#[inline(never)]
fn user_entry(argc: usize, argv: usize) {
    unsafe {
        let secondary_init: fn(usize, usize) = core::mem::transmute(ENTRY);
        // main_addr 表示用户进程 main 函数的地址
        secondary_init(argc, argv);
    }
    let start = get_time();

    poll_user_future();
    wait_other_cores();

    let end = get_time();
    println!("total time: {} ms", end - start);
    
    exit(0);
}


/// 各个进程的最高优先级协程，通过共享内存的形式进行通信
// pub static mut PRIO_ARRAY: [AtomicUsize; MAX_PROC_NUM + 1] = [const { AtomicUsize::new(usize::MAX) }; MAX_PROC_NUM + 1];
/// 进程内的优先级变化，某个优先级从 0->1 或 从 1-> 0，则更新对应的位置
pub static mut GLOBAL_BITMAP: [AtomicUsize; PRIO_NUM] = [const { AtomicUsize::new(0) }; PRIO_NUM];

/// 进程的 Executor 调用这个函数，只有在优先级发生变化时调用
#[no_mangle]
#[inline(never)]
fn update_prio(prio: usize, is_add: bool) {
    if is_add {
        unsafe { GLOBAL_BITMAP[prio].fetch_add(1, Ordering::SeqCst); }
    } else {
        unsafe { GLOBAL_BITMAP[prio].fetch_sub(1, Ordering::SeqCst); }
    }
}

/// poll_user_future 函数内部使用，当某个优先级变为 0 之后，调用这个函数判断是否需要让权
/// 对应的优先级计数不为 0，表示存在其他的进程还有这个优先级的协程，此时需要让权
fn shoule_yield(prio: usize) -> bool {
    for i in 0..prio {
        if unsafe { GLOBAL_BITMAP[i].load(Ordering::SeqCst) != 0 } {
            return true;
        }
    }
    return false;
}

/// 内核重新调度进程时，调用这个函数，选出优先级最高的进程，再选出对应的线程
/// 所有进程的优先级相同时，则内核会优先执行协程，这里用 0 来表示内核的优先级
#[no_mangle]
#[inline(never)]
pub fn max_prio() -> Option<usize> {
    for i in 0..PRIO_NUM {
        unsafe {
            if GLOBAL_BITMAP[i].load(Ordering::SeqCst) != 0 {
                return Some(i)
            }
        }
    }
    return None;
}


/// 添加协程，内核和用户态都可以调用
#[no_mangle]
#[inline(never)]
pub fn spawn(future: Pin<Box<dyn Future<Output=()> + 'static + Send + Sync>>, prio: usize, pid: usize, kind: CoroutineKind) -> usize {
    unsafe {
        let heapptr = *(HEAP_BUFFER as *const usize);
        let exe = (heapptr + core::mem::size_of::<LockedHeap>()) as *mut usize as *mut Executor;
        let (cid, need_update) = (*exe).spawn(future, prio, kind).unwrap();
        // 更新优先级标记
        if need_update {
            update_prio(prio, true);
        }
        return cid.0;
    }
}
/// 用户程序执行协程
#[no_mangle]
#[inline(never)]
pub fn poll_user_future() {
    unsafe {
        let heapptr = *(HEAP_BUFFER as *const usize);
        let exe = (heapptr + core::mem::size_of::<LockedHeap>()) as *mut usize as *mut Executor;
        let pid = getpid() as usize;
        let tid = gettid();
        loop {
            if (*exe).is_empty() {
                // println!("ex is empty");
                break;
            }
            if let Some((task, need_update)) = (*exe).fetch(tid as usize) {
                let prio = task.inner.lock().prio;
                let cid = task.cid;
                match task.execute() {
                    Poll::Pending => { }
                    Poll::Ready(()) => {
                        (*exe).del_coroutine(cid);
                    }
                };
                if need_update {
                    update_prio(prio, false);
                }
                if shoule_yield(prio) {
                    yield_();
                }
            } else {
                // 任务队列不为空，但就绪队列为空，等待任务唤醒
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
        let heapptr = *(HEAP_BUFFER as *const usize);
        let exe = (heapptr + core::mem::size_of::<LockedHeap>()) as *mut usize as *mut Executor;
        loop {
            if let Some((task, need_update)) = (*exe).fetch(hart_id()) {
                let cid = task.cid;
                let kind = task.kind;
                let prio = task.inner.lock().prio;
                match task.execute() {
                    Poll::Pending => {
                        if kind == CoroutineKind::KernSche {
                            // kprintln!("pending reback sche task{:?} kind {:?}", cid, kind);
                            re_back(cid.0, 0);
                        }
                    }
                    Poll::Ready(()) => {
                        (*exe).del_coroutine(cid);
                    }
                };
                // 尽管 re_back 可能会增加，但是也是属于 need_update 的情况，两次相互抵消
                if need_update {
                    // kprintln!("pending reback sche task{:?} kind {:?}", cid, kind);
                    update_prio(prio, false);
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
        let heapptr = *(HEAP_BUFFER as *const usize);
        let exe = (heapptr + core::mem::size_of::<LockedHeap>()) as *mut usize as *mut Executor;
        (*exe).currents[tid].as_mut().unwrap().get_val()
    }
}

/// 协程重新入队，手动执行唤醒的过程，内核和用户都会调用这个函数
#[no_mangle]
#[inline(never)]
pub fn re_back(cid: usize, pid: usize) {
    // println!("[Exec]re back func enter");
    unsafe {
        let heapptr = *(HEAP_BUFFER as *const usize);
        let exe = (heapptr + core::mem::size_of::<LockedHeap>()) as *mut usize as *mut Executor;
        let cid = CoroutineId(cid);
        let prio = (*exe).tasks.lock().get(&cid).unwrap().inner.lock().prio;
        let (cid, need_update) = (*exe).re_back(cid, prio).unwrap();
        // 重新入队之后，需要检查优先级
        if need_update {
            update_prio(prio, true)
        }
    }
}

/// 更新协程优先级
#[no_mangle]
#[inline(never)]
pub fn reprio(cid: usize, prio: usize) {
    unsafe {
        let heapptr = *(HEAP_BUFFER as *const usize);
        let exe = (heapptr + core::mem::size_of::<LockedHeap>()) as *mut usize as *mut Executor;
        (*exe).reprio(CoroutineId(cid), prio);
    }
}

/// 申请虚拟CPU
#[no_mangle]
#[inline(never)]
pub fn add_virtual_core() {
    unsafe {
        let heapptr = *(HEAP_BUFFER as *const usize);
        let exe = (heapptr + core::mem::size_of::<LockedHeap>()) as *mut usize as *mut Executor;
        let tid = thread_create(poll_user_future as usize, 0) as usize;
        (*exe).add_wait_tid(tid);
    }
}


pub fn wait_other_cores() {
    unsafe {
        let heapptr = *(HEAP_BUFFER as *const usize);
        let exe = (heapptr + core::mem::size_of::<LockedHeap>()) as *mut usize as *mut Executor;
        for tid in (*exe).waits.iter() {
            waittid(*tid);
        }
    }
}