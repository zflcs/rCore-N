// //! 这个库暴露出共享调度器中使用的数据结构以及接口
// //! 将 `Executor` 数据结构暴露出来，避免在内核和 user_lib 中重复定义
// //! 进程需要在自己的地址空间中声明这个对象
// //! 共享调度器通过 `Executor` 对象的虚拟地址来完成对应的操作

#![no_std]
#[warn(non_snake_case)]

extern crate alloc;
mod bitmap;
mod coroutine;
mod executor;
mod config;

// extern crate alloc;

pub use executor::Executor;
pub use coroutine::{CoroutineId, Coroutine, CoroutineKind};
use bitmap::BitMap;
pub use config::PRIO_NUM;