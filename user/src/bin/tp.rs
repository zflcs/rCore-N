
#![no_std]
#![no_main]

#[macro_use]
extern crate user_lib;
extern crate alloc;


use alloc::{vec::Vec, vec, string::{String, ToString}, boxed::Box};
use user_lib::{UintrFrame, uintr_register_receier, AwaitHelper, matrix::{string_to_matrix, matrix_multiply, matrix_to_string}};
use user_syscall::{close, listen, accept, aread, exit, uintr_create_fd, write};

use crossbeam::queue::ArrayQueue;

const MESSAGE_QUEUE_LEN: usize = 1 << 8;
type MessageQueue = ArrayQueue<String>;



const BUF_LEN: usize = 2048;
const MATRIX_SIZE: usize = 10;

const CLOSE_CONNECT_STR: &str = "close connection";

static MAX_POLL_THREADS: usize = 4 - 1;

const SERVER_USE_PRIO: usize = 2;
const CONNECTION_NUM: usize = SERVER_USE_PRIO * 1;

// request
static mut REQ_MAP: Vec<MessageQueue> = Vec::new();
// response
static mut RSP_MAP: Vec<MessageQueue> = Vec::new();

#[no_mangle]
pub fn main() -> i32 {

    println!("This is a very simple http server");
    uintr_init();
    
    let tcp_fd = listen(80);
    if tcp_fd < 0 {
        println!("Failed to listen on port 80");
        return -1;
    }
    init_connection();
    for i in 0..CONNECTION_NUM {
        let client_fd = accept(tcp_fd as usize);
        let send_rsp_cid = vdso::spawn(move || send_rsp_async(client_fd as usize, i), i % SERVER_USE_PRIO, executor::CoroutineKind::Norm);
        let matrix_calc_cid = vdso::spawn(move || matrix_calc_async(client_fd as usize, send_rsp_cid, i), i % SERVER_USE_PRIO, executor::CoroutineKind::Norm);
        vdso::spawn(move || handle_tcp_client_async(client_fd as usize, matrix_calc_cid, i), i % SERVER_USE_PRIO, executor::CoroutineKind::Norm);
    }
    // vdso::add_vcpu(MAX_POLL_THREADS);
    0
}

#[no_mangle]
pub extern "C" fn uintr_handler(_uintr_frame: &mut UintrFrame, irqs: usize) -> usize {
    println!("need wake up coroutine {}", irqs);
    vdso::re_back(irqs);
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

fn init_connection() {
    for _ in 0..(CONNECTION_NUM + 10) {
        unsafe {
            REQ_MAP.push(MessageQueue::new(MESSAGE_QUEUE_LEN));
            RSP_MAP.push(MessageQueue::new(MESSAGE_QUEUE_LEN));
        }
    }
}

async fn handle_tcp_client_async(client_fd: usize, matrix_calc_cid: usize, i: usize) {
    // println!("start tcp_client");
    let str: &str = "connect ok";
    let current_cid = vdso::current_cid(false);
    let mut begin_buf = vec![0u8; BUF_LEN];
    // async read requset
    aread(client_fd, begin_buf.as_mut(), current_cid).await;
    // sync write
    write(client_fd, str.as_bytes());
    println!("[{}] send ok", i);
    loop {
        let mut buf = vec![0u8; BUF_LEN];
        println!("[{}] aread from socket2", i);
        aread(client_fd, buf.as_mut(), current_cid).await;
        let recv_str: String = buf.iter()
            .take_while(|&&b| b != 0)
            .map(|&b| b as char)
            .collect();
        // println!("{:?}", recv_str);
        // save the request to requset queue
        unsafe { let _ = REQ_MAP[client_fd].push(recv_str.clone()); }
        println!("[{}] push req to requset queue", i);
        // wake up calculate coroutine
        if vdso::is_pending(matrix_calc_cid) {
            vdso::re_back(matrix_calc_cid);
        }
        if recv_str == CLOSE_CONNECT_STR {
            break;
        }
    }
    println!("[{}] handle_tcp_client_async end", i);
}

async fn matrix_calc_async(client_fd: usize, send_rsp_cid: usize, i: usize) {
    loop {
        println!("[{}] matrix cal run", i);
        let req_queue = unsafe { &mut REQ_MAP[client_fd] };
        if let Some(req) = req_queue.pop() {
            #[allow(unused_assignments)]
            let mut rsp = String::new();
            if req != CLOSE_CONNECT_STR {
                let matrix = string_to_matrix::<MATRIX_SIZE>(&req);
                let ans = matrix_multiply(matrix.clone(), matrix.clone());
                rsp = matrix_to_string(ans);
            } else {
                rsp = CLOSE_CONNECT_STR.to_string();
            }
            // save the response to response queue
            unsafe { let _ = RSP_MAP[client_fd].push(rsp); }
            // wake up send response coroutine
            if vdso::is_pending(send_rsp_cid) {
                vdso::re_back(send_rsp_cid);
            }
            if req == CLOSE_CONNECT_STR {
                break;
            }
        } else {    // there is no request message
            let mut helper = Box::new(AwaitHelper::new());
            helper.as_mut().await;
        }
    }
    println!("[{}] matrix_calc_async end", i);
}

async fn send_rsp_async(client_fd: usize, i: usize) {
    loop {
        println!("[{}] send_rsp run", i);
        let rsp_queue = unsafe { &mut RSP_MAP[client_fd] };
        if let Some(rsp) = rsp_queue.pop() {
            if rsp == CLOSE_CONNECT_STR {
                // println!("[send_rsp] break");
                // println!("close socket fd: {}", client_fd);
                close(client_fd);
                break;
            }
            println!("[{}] response len {}", i, rsp.len());
            write(client_fd, rsp.as_bytes());
        } else {
            let mut helper = Box::new(AwaitHelper::new());
            helper.as_mut().await;
        }
    }
    println!("[{}] send_rsp_async end", i);
}