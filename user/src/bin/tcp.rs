#![no_std]
#![no_main]

#[macro_use]
extern crate user_lib;
extern crate alloc;


use alloc::{vec, string::String};
use user_lib::{UintrFrame, uintr_register_receier};
use user_syscall::{close, listen, accept, aread, exit, uintr_create_fd, write};

const BUF_LEN: usize = 2048;

#[no_mangle]
pub fn main() -> i32 {

    println!("This is a very simple http server");
    uintr_init();

    
    let tcp_fd = listen(80);
    if tcp_fd < 0 {
        println!("Failed to listen on port 80");
        return -1;
    }
    let client_fd = accept(tcp_fd as usize);
    vdso::spawn(move || server(client_fd), 0, executor::CoroutineKind::Norm);
    vdso::spawn(test, 1, executor::CoroutineKind::Norm);    

    println!("add coroutine ok");
    0
}


async fn server(socket_fd: isize) {
    let mut begin_buf = vec![0u8; BUF_LEN];
    aread(socket_fd as usize, begin_buf.as_mut(), vdso::current_cid(false)).await;
    let recv_str: String = begin_buf.iter()
            .take_while(|&&b| b != 0)
            .map(|&b| b as char)
            .collect();
    println!("{:?}", recv_str);
    let str: &str = "connect ok";
    write(socket_fd as _, str.as_bytes());
    close(socket_fd as usize);
}

async fn test() {
    println!("this coroutine shoule run {}", vdso::current_cid(false));
}


#[no_mangle]
pub extern "C" fn uintr_handler(_uintr_frame: &mut UintrFrame, irqs: usize) -> usize {
    println!("\t-- User Interrupt handler --");
    // read pending bits
    println!("\tPending User Interrupts: {:b}", irqs);
    println!("need wake up coroutine {}", irqs);
    vdso::wake(None);
    return 0;
}

pub fn uintr_init() {
    if uintr_register_receier(uintr_handler as usize) != 0 {
        println!("Interrupt handler register error");
        exit(-1);
    }
    let uint_fd = uintr_create_fd(1);
    if uint_fd  < 0 {
        println!("Interrupt vector allocation error");
        exit(-2);
    }
    println!("Receiver enabled interrupts");
}


