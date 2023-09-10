#![no_std]
#![no_main]

#[macro_use]
extern crate user_lib;
extern crate alloc;


use alloc::vec;
use user_syscall::{close, read, listen, accept};

const BUF_LEN: usize = 2048;

#[no_mangle]
pub fn main() -> i32 {

    println!("This is a very simple http server");
    
    let tcp_fd = listen(80);
    if tcp_fd < 0 {
        println!("Failed to listen on port 80");
        return -1;
    }
    let client_fd = accept(tcp_fd as usize);

    let str = "connect ok";
    let mut begin_buf = vec![0u8; BUF_LEN];
    read(client_fd as usize, begin_buf.as_mut());
    for i in begin_buf {
        print!("{}", i as char);
    }
    println!("");
    close(client_fd as usize);
    println!("finish tcp test");
    0
}




