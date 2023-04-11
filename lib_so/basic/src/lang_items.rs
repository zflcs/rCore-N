use syscall::exit;

#[lang = "eh_personality"]
#[no_mangle]
pub extern fn rust_eh_personality() {
}

// #[no_mangle]
// pub extern fn memcpy() {
// }
// #[no_mangle]
// pub extern fn memmove() {
// }
// #[no_mangle]
// pub extern fn memset() {
// }
// #[no_mangle]
// pub extern fn _Unwind_Resume() {
// }
// #[no_mangle]
// pub extern fn bcmp() {
// }
// #[no_mangle]
// pub extern fn memcmp() {
// }
// #[no_mangle]
// pub extern fn strlen() {
// }

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

use core::{
    alloc::{GlobalAlloc, Layout},
};
use config::HEAP_BUFFER;
use buddy_system_allocator::LockedHeap;

/// 共享代码中默认的分配器，使用的是内核和用户程序各自的堆
/// 前提：堆的虚拟地址都保存在 HEAP_BUFFER 这个虚拟地址中
/// 分配和回收时，先读取 HEAP_BUFFER 虚拟地址中的内容
/// 再类型转换成正确的数据结构指针
/// 如果是把 heap 的指针当作参数传进需要使用的代码中，那么在分配的时候，需要显式的指出堆分配器
/// 通过这种方式，可以让默认的分配器使用不同的堆
#[global_allocator]
static GLOBAL: Global = Global;

struct Global;
unsafe impl GlobalAlloc for Global {
    #[inline]
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let heapptr = *(HEAP_BUFFER as *const usize);
        let heap = heapptr as *mut usize as *mut LockedHeap;
        (*heap).alloc(layout)
    }

    #[inline]
    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        let heapptr = *(HEAP_BUFFER as *const usize);
        let heap = heapptr as *mut usize as *mut LockedHeap;
        (*heap).dealloc(ptr, layout)
    }
}

// #[no_mangle]
// fn __rust_alloc(size: usize, align: usize) -> *mut u8 {
//     unsafe { 
//         let heapptr = *(HEAP_BUFFER as *const usize);
//         let heap = heapptr as *mut usize as *mut LockedHeap;
//         (*heap).alloc(Layout::from_size_align_unchecked(size, align))
//     }
// }

// #[no_mangle]
// fn __rust_dealloc(ptr: *mut u8, size: usize, align: usize) {
//     unsafe { 
//         let heapptr = *(HEAP_BUFFER as *const usize);
//         let heap = heapptr as *mut usize as *mut LockedHeap;
//         (*heap).dealloc(ptr, Layout::from_size_align_unchecked(size, align))
//     }
// }

// #[no_mangle]
// fn __rust_realloc(ptr: *mut u8, old_size: usize, align: usize, new_size: usize) -> *mut u8 {
//     unsafe { 
//         let heapptr = *(HEAP_BUFFER as *const usize);
//         let heap = heapptr as *mut usize as *mut LockedHeap;
//         (*heap).realloc(ptr, Layout::from_size_align_unchecked(old_size, align), new_size)
//     }
// }

// #[no_mangle]
// fn __rust_alloc_zeroed(size: usize, align: usize) -> *mut u8 {
//     unsafe { 
//         let heapptr = *(HEAP_BUFFER as *const usize);
//         let heap = heapptr as *mut usize as *mut LockedHeap;
//         (*heap).alloc_zeroed(Layout::from_size_align_unchecked(size, align))
//     }
// }