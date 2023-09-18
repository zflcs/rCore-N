#![no_std]
#![no_main]
#![feature(panic_info_message)]
#![feature(atomic_from_mut, inline_const)]
#![feature(alloc_error_handler)]
// #![deny(warnings, missing_docs)]


#[macro_use]
pub mod console;
#[macro_use]
pub mod kern_console;
pub mod config;

extern crate alloc;
use alloc::boxed::Box;
use bit_field::BitField;
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


#[alloc_error_handler]
pub fn handle_alloc_error(layout: core::alloc::Layout) -> ! {
    panic!("Heap allocation error, layout = {:?}", layout);
}

use user_syscall::*;

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
        INTERFACE[1] = spawn as usize;
        INTERFACE[2] = re_back as usize;
        INTERFACE[3] = current_cid as usize;
        INTERFACE[4] = is_pending as usize;
        INTERFACE[5] = add_vcpu as usize;
        &INTERFACE as *const [usize; 10] as usize
    }
}



use config::{ENTRY, MAX_THREAD, GLOBAL_BITMAP_BASE};
use executor::{Executor, CoroutineId, CoroutineKind, MAX_PRIO};

use core::task::Poll;


/// sret 进入用户态的入口，根据传递的堆指针，直接初始化堆
#[no_mangle]
#[inline(never)]
fn user_entry() {
    unsafe {
        let user_fn: fn() = core::mem::transmute(ENTRY);
        user_fn();
    }
    let heapptr = unsafe { *(HEAP_POINTER as *const usize) };
    let exe = (heapptr + core::mem::size_of::<Heap>()) as *mut usize as *mut Executor;
    let executor = unsafe { &mut *exe };
    let tid = gettid() as usize;
    loop {
        if let Some(task) = executor.fetch(tid) {
            let task_clone = task.clone();
            match task.execute() {
                Poll::Pending => {
                    executor.pending(task_clone);
                }
                Poll::Ready(()) => {
                    executor.remove(task_clone);
                }
            };
            executor.update_state(tid);
        } else {
            if executor.is_empty() {
                break;
            } else {
                sleep(0.5);
            }
        }
    }
    wait_vcpu();
    exit(0);
}



/// 添加协程，内核和用户态都可以调用
#[no_mangle]
#[inline(never)]
pub fn spawn(future: Pin<Box<dyn Future<Output=()> + 'static + Send + Sync>>, prio: usize, kind: CoroutineKind) -> usize {
    unsafe {
        let heapptr = *(HEAP_POINTER as *const usize);
        let exe = (heapptr + core::mem::size_of::<Heap>()) as *mut usize as *mut Executor;
        let cid = (*exe).spawn(future, prio, kind);
        return cid.0;
    }
}


/// 用户程序执行协程
#[no_mangle]
#[inline(never)]
pub fn poll_user_future() {
    let heapptr = unsafe { *(HEAP_POINTER as *const usize) };
    let exe = (heapptr + core::mem::size_of::<Heap>()) as *mut usize as *mut Executor;
    let executor = unsafe { &mut *exe };
    let tid = gettid() as usize;
    loop {
        if let Some(task) = executor.fetch(tid) {
            let task_clone = task.clone();
            match task.execute() {
                Poll::Pending => {
                    executor.pending(task_clone);
                }
                Poll::Ready(()) => {
                    executor.remove(task_clone);
                }
            };
            executor.update_state(tid);
        } else {
            if executor.is_empty() {
                break;
            } else {
                sleep(0.5);
            }
        }
    }
    exit(0);
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

/// 获取当前正在执行的协程 id
#[no_mangle]
#[inline(never)]
pub fn current_cid(is_kernel: bool) -> usize {
    let tid = if is_kernel { hart_id() } else {
        gettid() as usize
    };
    assert!(tid < MAX_THREAD);
    unsafe {
        let heapptr = *(HEAP_POINTER as *const usize);
        let exe = (heapptr + core::mem::size_of::<Heap>()) as *mut usize as *mut Executor;
        (*exe).cur(tid).0
    }
}

/// 协程重新入队，手动执行唤醒的过程，内核和用户都会调用这个函数
#[no_mangle]
#[inline(never)]
pub fn re_back(cid: usize) {
    unsafe {
        let heapptr = *(HEAP_POINTER as *const usize);
        let exe = (heapptr + core::mem::size_of::<Heap>()) as *mut usize as *mut Executor;
        (*exe).re_back(CoroutineId(cid));
    }
}

#[no_mangle]
#[inline(never)]
pub fn is_pending(cid: usize) -> bool {
    unsafe {
        let heapptr = *(HEAP_POINTER as *const usize);
        let exe = (heapptr + core::mem::size_of::<Heap>()) as *mut usize as *mut Executor;
        return (*exe).is_pending(cid)
    }
}


/// 申请虚拟CPU
#[no_mangle]
#[inline(never)]
pub fn add_vcpu(vcpu_num: usize) {
    unsafe {
        let heapptr = *(HEAP_POINTER as *const usize);
        let exe = (heapptr + core::mem::size_of::<Heap>()) as *mut usize as *mut Executor;
        for _ in 0..vcpu_num {
            let tid = thread_create(poll_user_future as usize) as usize;
            (*exe).add_wait_tid(tid);
        }
    }
}


pub fn wait_vcpu() {
    unsafe {
        let heapptr = *(HEAP_POINTER as *const usize);
        let exe = (heapptr + core::mem::size_of::<Heap>()) as *mut usize as *mut Executor;
        for _ in 0..(*exe).waits.len() {
            let mut exit_code = 0;
            wait(&mut exit_code);
        }
    }
}

pub fn check_yield(prio: usize) -> bool {
    let global_bitmap = unsafe { (GLOBAL_BITMAP_BASE as *const usize).read() };
    for i in 0..MAX_PRIO {
        if global_bitmap.get_bit(i) && i < prio{
            return true;
        }
    }
    false
}
