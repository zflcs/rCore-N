use alloc::collections::VecDeque;
use alloc::sync::Arc;
use alloc::vec::Vec;
use executor::Coroutine;
use lazy_static::lazy_static;
use spin::Mutex;
use crate::task::{Task, TASK_MANAGER, Scheduler, TaskState};
use smoltcp::wire::*;

// TODO: specify the protocol, TCP or UDP
pub struct Socket {
    pub raddr: Ipv4Address,                // remote address
    pub lport: u16,                 // local port
    pub rport: u16,                 // rempote port
    pub buffers: VecDeque<Vec<u8>>, // datas
    pub seq: TcpSeqNumber,
    pub ack: Option<TcpSeqNumber>,
    pub block_task: Option<Arc<Task>>,
    pub block_coroutine: Option<Arc<Coroutine>>,
}

const MAX_SOCKETS_NUM: usize = 512;

lazy_static! {
    static ref SOCKET_TABLE: Mutex<Vec<Option<Arc<Mutex<Socket>>>>> = Mutex::new(Vec::with_capacity(MAX_SOCKETS_NUM));
}

pub fn get_mutex_socket(index: usize) -> Option<Arc<Mutex<Socket>>> {
    let socket_table = SOCKET_TABLE.lock();
    socket_table.get(index).map_or(None, |x| (*x).clone())
}

pub fn get_s_a_by_index(index: usize) -> Option<(TcpSeqNumber, Option<TcpSeqNumber>)> {
    let socket_table = SOCKET_TABLE.lock();

    assert!(index < socket_table.len());

    socket_table.get(index).map_or(None, |x| match x {
        Some(x) => {
            let socket = x.lock();
            return Some((socket.seq, socket.ack));
        }
        None => None
    })
}


pub fn get_socket(raddr: Ipv4Address, lport: u16, rport: u16) -> Option<usize> {
    log::trace!("search raddr {:?}, lport {}, rport {}", raddr, lport, rport);
    let socket_table = SOCKET_TABLE.lock();
    for i in 0..socket_table.len() {
        let sock = &socket_table[i];
        if sock.is_none() {
            continue;
        }

        let sock = sock.as_ref().unwrap().lock();
        log::trace!("socket raddr {:?}, lport {}, rport {}", sock.raddr, sock.lport, sock.rport);
        if sock.raddr == raddr && sock.lport == lport && sock.rport == rport {
            return Some(i);
        }
    }
    None
}


pub fn add_socket(raddr: Ipv4Address, lport: u16, rport: u16, seq: TcpSeqNumber, ack: Option<TcpSeqNumber>) -> Option<usize> {
    if get_socket(raddr, lport, rport).is_some() {
        return None;
    }
    let mut socket_table = SOCKET_TABLE.lock();
    let mut index = usize::MAX;
    for i in 0..socket_table.len() {
        if socket_table[i].is_none() {
            index = i;
            break;
        }
    }

    let socket = Socket {
        raddr,
        lport,
        rport,
        buffers: VecDeque::new(),
        seq,
        ack,
        block_task: None,
        block_coroutine: None,
    };

    if index == usize::MAX {
        socket_table.push(Some(Arc::new(Mutex::new(socket))));
        Some(socket_table.len() - 1)
    } else {
        socket_table[index] = Some(Arc::new(Mutex::new(socket)));
        Some(index)
    }
}

pub fn remove_socket(index: usize) {
    let mut socket_table = SOCKET_TABLE.lock();

    assert!(socket_table.len() > index);
    socket_table[index] = None;
}

pub fn push_data(index: usize, packet: &TcpPacket<&[u8]>) {
    let mut socket_table = SOCKET_TABLE.lock();
    if socket_table.len() <= index || socket_table[index].is_none() {
        return;
    }
    assert!(socket_table.len() > index);
    assert!(socket_table[index].is_some());
    let mut socket = socket_table[index].as_mut().unwrap().lock();
    socket.ack = Some(packet.seq_number() + packet.segment_len());
    socket.seq = packet.ack_number();
    socket.buffers.push_back(packet.payload().to_vec());
    log::trace!("[push_data] index: {}, socket.ack:{:?}, socket.seq:{}", index, socket.ack, socket.seq);
    if let Some(coroutine) = socket.block_coroutine.take() {        // aread
        let cid = coroutine.cid;
        log::trace!("wake up coroutine {:?}", cid);
        let _ = TASK_MANAGER.lock().add(crate::task::KernTask::Corou(coroutine));
    } else {
        if let Some(task) = socket.block_task.take() {
            log::trace!("wake read task");
            task.locked_inner().state = TaskState::RUNNABLE;
            let _ = TASK_MANAGER.lock().add(crate::task::KernTask::Proc(task));
        }
    }
}