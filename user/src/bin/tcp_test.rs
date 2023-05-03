#![no_std]
#![no_main]
#[macro_use]
extern crate alloc;
extern crate user_lib;
use user_lib::*;
use alloc::string::String;
#[derive(PartialEq, Eq)]
enum ModelType {
    Coroutine = 1,
    Thread = 2,
}

static MAX_POLL_THREADS: usize = 1;
static MODEL_TYPE: ModelType = ModelType::Coroutine;
static CONNECTION_NUM: usize = 32;


fn handle_tcp_client(client_fd: usize) -> bool {
    println!("start tcp_client");
    let str = "connect ok";
    let mut begin_buf = vec![0u8; 1024];
    read!(client_fd as usize, &mut begin_buf);
    syscall::write!(client_fd, str.as_bytes());
    loop {
        let mut buf = vec![0u8; 1024];
        let _len = read!(client_fd as usize, &mut buf);
        let recv_str: String = buf.iter()
        .take_while(|&&b| b != 0)
        .map(|&b| b as char)
        .collect();
        if recv_str == "close connection" {
            break;
        }
        
        let responese = "response from server";
        // write a response
        syscall::write!(client_fd, responese.as_bytes());
    }
    
    close(client_fd);
    exit(2);
}


async fn handle_tcp_client_async(client_fd: usize) {
    println!("start tcp_client");
    let str = "connect ok";
    let mut begin_buf = vec![0u8; 1024];
    read!(client_fd as usize, &mut begin_buf, 0, current_cid());
    syscall::write!(client_fd, str.as_bytes());
    loop {
        let mut buf = vec![0u8; 1024];
        read!(client_fd as usize, &mut buf, 0, current_cid());
        let recv_str: String = buf.iter()
        .take_while(|&&b| b != 0)
        .map(|&b| b as char)
        .collect();
        if recv_str == "close connection" {
            break;
        }
        
        let responese = "response from server";
        // write a response
        syscall::write!(client_fd, responese.as_bytes());
    }
    close(client_fd);
}

#[no_mangle]
pub fn main() -> i32 {

    println!("This is a very simple http server");
    let pid = getpid();
    if MODEL_TYPE == ModelType::Coroutine {
        let init_res = init_user_trap();
        for _ in 0..MAX_POLL_THREADS {
            add_virtual_core();
        }
        println!(
            "[hello tcp test] trap init result: {:#x}, pid: {}",
            init_res, pid
        );
    }
    
    let tcp_fd = listen(80);
    if tcp_fd < 0 {
        println!("Failed to listen on port 80");
        return -1;
    }
    let mut wait_tid = vec![];
    for _ in 0..CONNECTION_NUM {
        let client_fd = accept(tcp_fd as usize);
        println!("client connected: {}", client_fd);
        if MODEL_TYPE == ModelType::Thread {
            let tid = thread_create(handle_tcp_client as usize, client_fd as usize) as usize;
            wait_tid.push(tid);
        } else {
            spawn(move || handle_tcp_client_async(client_fd as usize), 0);
        }
    }

    for tid in wait_tid.iter() {
        waittid(*tid);
    }

    println!("finish tcp test");
    0
}


#[no_mangle]
pub fn wake_handler(cid: usize) {
    re_back(cid);
}