#![no_std]
#![no_main]

#[macro_use]
extern crate user_lib;
extern crate alloc;

use user_lib::getpid;
use user_syscall::{uintr_register_receiver, uintr_create_fd, fork, uintr_register_sender, waitpid, exit};

#[no_mangle]
pub fn main() -> i32 {
    println!("[hello world] from pid: {}", getpid());
    uintr_register_receiver();
    let fd = uintr_create_fd(0x0);
    let pid = fork(17 | 256);
    if pid == 0 {       // child process
        uintr_register_sender(fd);
        println!("here");
        exit(0);
    } else {
        let mut exit_code = 0;
        waitpid(pid as usize, &mut exit_code);
    }
    0
}
