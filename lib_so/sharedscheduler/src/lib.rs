//! Executor 运行时
#![no_std]
#![feature(lang_items, core_intrinsics)]
#![feature(start)]
#![no_builtins]

use alloc::vec::Vec;
use config::{ENTRY, MAX_THREAD_NUM, HEAP_BUFFER};
use alloc::boxed::Box;
use basic::{CoroutineId, Executor, CoroutineKind};
use core::pin::Pin;
use core::future::Future;
use syscall::*;
use core::task::Poll;
use buddy_system_allocator::LockedHeap;
use basic::println;

extern crate alloc;
core::arch::global_asm!(include_str!("info.asm"));

#[no_mangle]
#[inline(never)]
pub extern "C" fn init_module() -> usize {
    let v = Vec::<usize>::new();
    v.as_ptr() as _
}

/// sret 进入用户态的入口，在这个函数再执行 main 函数
#[no_mangle]
#[inline(never)]
pub extern "C" fn user_entry(argc: usize, argv: usize) {
    assert_eq!(1, 2);
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
pub extern "C" fn spawn(future: Pin<Box<dyn Future<Output=()> + 'static + Send + Sync>>, prio: usize, kind: CoroutineKind) -> usize {
    unsafe {
        let heapptr = *(HEAP_BUFFER as *const usize);
        let exe = (heapptr + core::mem::size_of::<LockedHeap>()) as *mut usize as *mut Executor;
        // return 0;
        let cid = (*exe).spawn(future, prio, kind);
        return cid;
    }
}
/// 用户程序执行协程
#[no_mangle]
#[inline(never)]
pub extern "C" fn poll_user_future() {
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
pub extern "C" fn poll_kernel_future() {
    unsafe {
        let heapptr = *(HEAP_BUFFER as *const usize);
        let exe = (heapptr + core::mem::size_of::<LockedHeap>()) as *mut usize as *mut Executor;
        loop {
            let task = (*exe).fetch(hart_id());
            // 更新优先级标记
            // kprintln!("executor prio {}", 1);
            match task {
                Some(task) => {
                    let cid = task.cid;
                    let kind = task.kind;
                    match task.execute() {
                        Poll::Pending => {
                            if kind == CoroutineKind::KernSche {
                                // println_hart!("pending reback sche task{:?} kind {:?}", hart_id(), cid, kind);
                                wake(cid.0);
                            }
                        }
                        Poll::Ready(()) => {
                            (*exe).del_coroutine(cid);
                        }
                    };
                }
                _ => {
                    break;
                }
            }
        }
    }
}
/// 获取当前正在执行的协程 id
#[no_mangle]
#[inline(never)]
pub extern "C" fn current_cid(is_kernel: bool) -> usize {
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
pub extern "C" fn wake(cid: usize) {
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
pub extern "C" fn reprio(cid: usize, prio: usize) {
    unsafe {
        let heapptr = *(HEAP_BUFFER as *const usize);
        let exe = (heapptr + core::mem::size_of::<LockedHeap>()) as *mut usize as *mut Executor;
        (*exe).reprio(CoroutineId(cid), prio);
    }
}

/// 申请虚拟CPU
#[no_mangle]
#[inline(never)]
pub extern "C" fn add_virtual_core() {
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