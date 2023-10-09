#![no_std]
#![no_main]
#[macro_use]
extern crate alloc;
extern crate user_lib;
use user_lib::*;



#[no_mangle]
pub fn main() -> i32 {
    println!("This is a very simple http server");
    let init_res = init_uintr_trap();
    println!("Enabled user interrupts, trap_info_base {:#x}", init_res);
    let tcp_fd = listen(80);
    if tcp_fd < 0 {
        println!("Failed to listen on port 80");
        return -1;
    }
    spawn(move || read_test(tcp_fd as _), 0);
    spawn(|| hello(), 1);
    0
}

async fn read_test(tcp_fd: usize) {
    let mut buf = vec![0u8; 11];
    let current_cid = current_cid();
    read!(tcp_fd as _, &mut buf, 0, current_cid);
    println!("{:#x?}", buf);
}

async fn hello() {
    println!("this should print first");
}

#[no_mangle]
pub fn wake_handler(cid: usize) {
    re_back(cid);
}

// #[no_mangle]
// pub fn main() -> i32 {
//     let tcp_fd = listen(80);
//     if tcp_fd < 0 {
//         println!("Failed to listen on port 80");
//         return -1;
//     }
//     let mut buf = vec![0u8; 5];
//     for i in 0..3 {
//         read!(tcp_fd as _, &mut buf);
//         println!("{:#x?}", buf);
//     }
    
//     let msg = "connect close!!!";
//     syscall::write!(tcp_fd as _, msg.as_bytes());
//     syscall::write!(tcp_fd as _, msg.as_bytes());
//     sleep(500);
//     println!("finish tcp test");
//     close(tcp_fd as _);
//     0
// }


