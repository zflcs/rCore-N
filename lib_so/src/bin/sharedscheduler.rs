//! 共享调度器模块

// #![no_std]
#![no_main]
#![feature(inline_const, linkage)]

#[macro_use]
extern crate lib_so;
extern crate alloc;

use lib_so::config::{ENTRY, MAX_THREAD_NUM, HEAP_BUFFER};
use lib_so::{Executor, CoroutineId, CoroutineKind};
use alloc::boxed::Box;
use core::pin::Pin;
use core::future::Future;
use syscall::*;
use core::task::Poll;
use buddy_system_allocator::LockedHeap;


// 自定义的模块接口，模块添加进地址空间之后，需要执行 _start() 函数填充这个接口表
static mut INTERFACE: [usize; 10] = [0; 10];

#[no_mangle]
fn main() -> usize{
    unsafe {
        INTERFACE[0] = user_entry as usize;
        INTERFACE[2] = spawn as usize;
        INTERFACE[3] = poll_kernel_future as usize;
        INTERFACE[4] = wake as usize;
        INTERFACE[5] = current_cid as usize;
        INTERFACE[6] = reprio as usize;
        INTERFACE[7] = add_virtual_core as usize;
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


/// 添加协程，内核和用户态都可以调用
#[no_mangle]
#[inline(never)]
pub fn spawn(future: Pin<Box<dyn Future<Output=()> + 'static + Send + Sync>>, prio: usize, pid: usize, kind: CoroutineKind) -> usize {
    unsafe {
        let heapptr = *(HEAP_BUFFER as *const usize);
        let exe = (heapptr + core::mem::size_of::<LockedHeap>()) as *mut usize as *mut Executor;
        let cid = (*exe).spawn(future, prio, kind);
        return cid;
    }
}
/// 用户程序执行协程
#[no_mangle]
#[inline(never)]
pub fn poll_user_future() {
    unsafe {
        let heapptr = *(HEAP_BUFFER as *const usize);
        let exe = (heapptr + core::mem::size_of::<LockedHeap>()) as *mut usize as *mut Executor;
        let tid = gettid();
        loop {
            if (*exe).is_empty() {
                break;
            }
            let task = (*exe).fetch(tid as usize);
            match task {
                Some(task) => {
                    let cid = task.cid;
                    // println!("user task kind {:?}", task.kind);
                    match task.execute() {
                        Poll::Pending => { }
                        Poll::Ready(()) => {
                            (*exe).del_coroutine(cid);
                        }
                    };
                }
                _ => {
                    // 任务队列不为空，但就绪队列为空，等待任务唤醒
                    yield_();
                }
            }
            // 执行完优先级最高的协程，检查优先级，判断是否让权
            yield_();
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
            let task = (*exe).fetch(hart_id());
            // 更新优先级标记
            // println_hart!("executor prio {}", hart_id(), prio);
            match task {
                Some(task) => {
                    let cid = task.cid;
                    let kind = task.kind;
                    match task.execute() {
                        Poll::Pending => {
                            if kind == CoroutineKind::KernSche {
                                // println_hart!("pending reback sche task{:?} kind {:?}", hart_id(), cid, kind);
                                wake(cid.0, 0);
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
        let heapptr = *(HEAP_BUFFER as *const usize);
        let exe = (heapptr + core::mem::size_of::<LockedHeap>()) as *mut usize as *mut Executor;
        (*exe).currents[tid].as_mut().unwrap().get_val()
    }
}

/// 协程重新入队，手动执行唤醒的过程，内核和用户都会调用这个函数
#[no_mangle]
#[inline(never)]
pub fn wake(cid: usize, pid: usize) {
    // println!("[Exec]re back func enter");
    unsafe {
        let heapptr = *(HEAP_BUFFER as *const usize);
        let exe = (heapptr + core::mem::size_of::<LockedHeap>()) as *mut usize as *mut Executor;
        (*exe).wake(CoroutineId(cid));
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
        for tid in (*exe).waits.lock().iter() {
            waittid(*tid);
        }
    }
}
