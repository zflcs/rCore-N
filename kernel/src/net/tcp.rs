
use alloc::boxed::Box;
use alloc::vec;
use executor::Coroutine;
use ubuf::UserBuffer;
use vfs::File;
use crate::arch::uintr::uirs_send;
use crate::driver::net::NetDevice;
use crate::net::NET_STACK;

use crate::net::reply::build_eth_frame;
use crate::net::reply::build_eth_repr;
use crate::net::reply::build_ipv4_repr;
use crate::net::reply::build_tcp_repr;
use crate::task::Scheduler;
use crate::task::TASK_MANAGER;
use crate::task::cpu;
use crate::task::do_block;
use crate::task::KernTask;

use super::socket::get_mutex_socket;
use super::socket::{add_socket, get_s_a_by_index, remove_socket};

use smoltcp::wire::*;


pub struct TCP {
    pub src_mac: EthernetAddress,
    pub src_ip: Ipv4Address,
    pub dst_ip: Ipv4Address,
    pub src_port: u16,
    pub dst_port: u16,
    pub seq_number: TcpSeqNumber,
    pub ack_number: Option<TcpSeqNumber>,
    pub socket_index: usize,
}

impl TCP {
    pub fn new(
        src_mac: EthernetAddress,
        src_ip: Ipv4Address, 
        dst_ip: Ipv4Address, 
        src_port: u16, 
        dst_port: u16, 
        seq_number: TcpSeqNumber, 
        ack_number: Option<TcpSeqNumber>
    ) -> Option<Self> {
        if let Some(socket_index) = add_socket(src_ip, src_port, dst_port, seq_number, ack_number) {
            Some(Self {
                src_mac,
                src_ip,
                dst_ip,
                src_port,
                dst_port,
                seq_number,
                ack_number,
                socket_index,
            })
        } else {
            None
        }
    }
}


impl File for TCP {
    fn readable(&self) -> bool {
        true
    }

    fn writable(&self) -> bool {
        true
    }

    fn areadable(&self) -> bool {
        true
    }

    fn aread(&self, buf: UserBuffer, cid: usize) -> Option<usize> {
        if !self.areadable() {
            return None;
        }
        let cur = cpu().curr.as_ref().unwrap();
        let coroutine = Coroutine::new(
            Box::pin(aread_coroutine(self.socket_index, buf, cid)), 
            0, 
            executor::CoroutineKind::Norm
        );
        log::debug!("cid {:?}", coroutine.cid);
        let socket = get_mutex_socket(self.socket_index).unwrap();
        socket.lock().block_task = Some(cur.clone());
        socket.lock().block_coroutine = Some(coroutine.clone());
        log::debug!("aread from tcp");
        let _ = TASK_MANAGER.lock().add(KernTask::Corou(coroutine));
        Some(0)
    }

    fn read(&self, buf: &mut [u8]) -> Option<usize> {
        let socket = get_mutex_socket(self.socket_index).unwrap();
        log::debug!("read from tcp");
        loop {
            let mut mutex_socket = socket.lock();
            if let Some(data) = mutex_socket.buffers.pop_front() {
                drop(mutex_socket);
                let data_len = data.len();
                let mut count = 0;
                buf[0..data_len].copy_from_slice(&data);
                count += data_len;
                return Some(count);
            } else {
                let cur = cpu().curr.as_ref().unwrap();
                mutex_socket.block_task = Some(cur.clone());
                drop(mutex_socket);
                unsafe{ do_block(); }
            }
        }
    }

    fn write(&self, buf: &[u8]) -> Option<usize> {
        let mut data = vec![0u8; buf.len()];
        let mut count = 0;
        data.copy_from_slice(buf);
        count += buf.len();
        log::debug!("socket send len: {}", count);
        // get sock and sequence
        let (seq_number, ack_number) = get_s_a_by_index(self.socket_index)
            .map_or((TcpSeqNumber::default(), None), |x| x);
        let tcp_repr = build_tcp_repr(
            self.dst_port, 
            self.src_port, 
            TcpControl::Psh, 
            seq_number, 
            ack_number, 
            buf
        );
        let src_ip = self.dst_ip;
        let dst_ip = self.src_ip;
        let ipv4_repr = build_ipv4_repr(src_ip, dst_ip, IpProtocol::Tcp, tcp_repr.buffer_len());
        let eth_repr = build_eth_repr(NET_STACK.mac_addr, self.src_mac, EthernetProtocol::Ipv4);
        if let Some(eth_frame) = build_eth_frame(eth_repr, None, Some((ipv4_repr, tcp_repr))) {
            log::debug!("write tcp socket ok1");
            NetDevice.transmit(&eth_frame.into_inner());
        }
        log::debug!("write tcp socket ok2");
        Some(count)
    }
}

impl Drop for TCP {
    fn drop(&mut self) {
        remove_socket(self.socket_index)
    }
}

async fn aread_coroutine(socket_index: usize, mut buf: UserBuffer, cid: usize) {
    let socket = get_mutex_socket(socket_index).unwrap();
    loop {
        let mut mutex_socket = socket.lock();
        if let Some(data) = mutex_socket.buffers.pop_front() {
            drop(mutex_socket);
            let data_len = data.len();
            log::debug!("read len {:?}", data_len);
            let mut left = 0;
            for i in 0..buf.inner.len() {
                let buffer_i_len = buf.inner[i].len().min(data_len - left);
                buf.inner[i][..buffer_i_len]
                    .copy_from_slice(&data[left..(left + buffer_i_len)]);
                left += buffer_i_len;
                if left == data_len {
                    break;
                }
            }
            break;
        } else {    // await the current coroutine
            drop(mutex_socket);
            // leaf future await
            ReadHelper::new().await;
        }
    }
    // send user interrupt
    let task = socket.lock().block_task.take().unwrap();
    log::debug!("send user interrupt {}", cid);
    unsafe { uirs_send(task, cid) };
}

use core::{future::Future, pin::Pin, task::{Poll, Context}};


pub struct ReadHelper(usize);

impl ReadHelper {
    pub fn new() -> Self {
        Self(0)
    }
}

impl Future for ReadHelper {
    type Output = ();

    fn poll(mut self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Self::Output> {
        self.0 += 1;
        if (self.0 & 1) == 1 {
            return Poll::Pending;
        } else {
            return Poll::Ready(());
        }
    }
}