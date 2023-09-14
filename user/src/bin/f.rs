
#![no_std]
#![no_main]

#[macro_use]
extern crate user_lib;

use user_lib::{exit, fork, get_time, getpid, sleep, wait};

static NUM: usize = 1;

#[no_mangle]
pub fn main() -> i32 {
    println!("forktest2=========");
    for _ in 0..NUM {
        let pid = fork();
        if pid == 0 {
            sleep(1.0);
            println!("pid {} OK!", getpid());
            exit(0);
        }
    }

    let mut exit_code: i32 = 0;
    for _ in 0..NUM {
        let res = wait(&mut exit_code);
        println!("wait res {}", res);
        assert!(res > 0);
        assert_eq!(exit_code, 0);
    }
    assert!(wait(&mut exit_code) < 0);
    println!("forktest2 test passed!");
    0
}
