
#![no_std]
#![no_main]

#[macro_use]
extern crate user_lib;

// use user_lib::{fork, getpid, sleep, wait};

// static NUM: usize = 5;

// #[no_mangle]
// pub fn main() -> i32 {
//     println!("forktest2=========");
//     for _ in 0..NUM {
//         let pid = fork();
//         if pid == 0 {
//             sleep(1.0);
//             println!("pid {} OK!", getpid());
//         } else {
//             let mut exit_code: i32 = 0;
//             let res = wait(&mut exit_code);
//             println!("wait res {}", res);
//             assert!(res > 0);
//             assert_eq!(exit_code, 0);
//         }
//     }
//     0
// }

extern crate alloc;
use user_syscall::{exit, sleep, thread_create, thread_join};

static NUM: usize = 1;

#[no_mangle]
pub fn main() -> i32 {
    println!("forktest2=========");
    let tid = thread_create(thread as usize);
    thread_join(tid);
    // println!("tid {} OK!", gettid());
    0
}



pub fn thread() {
    println!("thread {:#x?}, Num {}", thread as usize, NUM);
    exit(0);
}
