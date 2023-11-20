#![allow(unused_assignments)]

extern crate alloc;
extern crate proc_macro;
use proc_macro::TokenStream;
use quote::{quote, ToTokens};
use syn::{parse_macro_input, ItemFn};

#[proc_macro_attribute]
pub fn main(_attr: TokenStream, item: TokenStream) -> TokenStream {
    let input_fn = parse_macro_input!(item as ItemFn);
    let blocks = input_fn.block.stmts;
    let mut statements = Vec::new();
    for stmt in blocks {
        statements.push(stmt.clone().to_token_stream());
    }
    // println!("{:?}", args_value[0]);
    let mut derive_fn = TokenStream::default();
    derive_fn = quote!(
        #[macro_use]
        extern crate lang;
        extern crate alloc;
        use alloc::boxed::Box;
        use core::future::Future;
        extern crate syscall;
        use syscall::*;

        #[no_mangle]
        pub fn main() -> Box<dyn Future<Output = i32> + 'static + Send + Sync> {
            init_heap();
            lang::console::init(option_env!("LOG"));
            init_executor();
            Box::new(main_fut())
        }

        #[no_mangle]
        pub extern "C" fn put_str(ptr: *const u8, len: usize) {
            sys_write(1, ptr as _, len, usize::MAX, usize::MAX);
        }

        pub fn getchar() -> u8 {
            let mut c = [0u8; 1];
            let mut res = -1;
            while res < 0 {
                res = sys_read(0, c.as_ptr() as usize, c.len(), usize::MAX, usize::MAX);
            }
            c[0]
        }


        use buddy_system_allocator::LockedHeap;
        use core::{
            alloc::Layout,
            ptr::NonNull,
        };
        use executor::Executor;
        use spin::Once;


        #[no_mangle]
        #[link_section = ".data.heap"]
        pub static mut HEAP: LockedHeap<32> = LockedHeap::new();


        #[no_mangle]
        #[link_section = ".data.executor"]
        pub static mut EXECUTOR: Once<Executor> = Once::new();

        pub const USER_HEAP_SIZE: usize = 0x40000;

        #[no_mangle]
        #[link_section = ".bss.memory"]
        static mut MEMORY: [u8; USER_HEAP_SIZE] = [0u8; USER_HEAP_SIZE];

        /// 
        fn init_heap() {
            unsafe {
                HEAP.lock().init(MEMORY.as_ptr() as usize, USER_HEAP_SIZE);
            }
        }

        /// init
        fn init_executor() {
            unsafe {
                EXECUTOR.call_once(|| Executor::new());
            }
        }

        #[no_mangle]
        pub unsafe extern "C" fn alloc(size: usize, align: usize) -> *mut u8 {
            HEAP.lock()
                .alloc(Layout::from_size_align_unchecked(size, align))
                .ok()
                .map_or(0 as *mut u8, |allocation| allocation.as_ptr())
        }

        #[no_mangle]
        pub unsafe extern "C" fn dealloc(ptr: *mut u8, size: usize, align: usize) {
            HEAP.lock().dealloc(
                NonNull::new_unchecked(ptr), 
                Layout::from_size_align_unchecked(size, align)
            )
        }
        pub async fn main_fut() -> i32 {
            #(#statements)*
        }
    ).into();
    // println!("{}", derive_fn.to_string());
    derive_fn
}
