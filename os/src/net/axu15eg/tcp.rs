
use alloc::boxed::Box;
use alloc::vec;
use smoltcp::wire::EthernetAddress;
use smoltcp::wire::EthernetProtocol;
use smoltcp::wire::IpProtocol;
use smoltcp::wire::Ipv4Address;
use smoltcp::wire::TcpControl;
use smoltcp::wire::TcpSeqNumber;
use crate::device::net::NetDevice;
use crate::fs::File;
use crate::mm::UserBuffer;
use crate::net::NET_STACK;

use crate::net::axu15eg::reply::build_eth_frame;
use crate::net::axu15eg::reply::build_eth_repr;
use crate::net::axu15eg::reply::build_ipv4_repr;
use crate::net::axu15eg::reply::build_tcp_repr;
use crate::task::block_current_and_run_next;
use crate::task::current_task;
use crate::trap::UserTrapRecord;
use crate::trap::push_message;

use super::ASYNC_RDMP;
use super::socket::get_mutex_socket;
use super::socket::{add_socket, get_s_a_by_index, remove_socket};



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

    fn awrite(&self, buf: crate::mm::UserBuffer, pid: usize, key: usize) -> core::pin::Pin<alloc::boxed::Box<dyn core::future::Future<Output = ()> + 'static + Send + Sync>> {
        todo!()
    }

    fn aread(&self, mut buf: crate::mm::UserBuffer, cid: usize, pid: usize, key: usize) -> core::pin::Pin<alloc::boxed::Box<dyn core::future::Future<Output = ()> + 'static + Send + Sync>> {
        Box::pin(async_read(self.socket_index, buf, cid, pid))
    }

    fn read(&self, mut buf: crate::mm::UserBuffer) -> Result<usize, isize> {
        let socket = get_mutex_socket(self.socket_index).unwrap();
        log::trace!("read from tcp");
        loop {
            let mut mutex_socket = socket.lock();
            if let Some(data) = mutex_socket.buffers.pop_front() {
                drop(mutex_socket);
                let data_len = data.len();
                let mut left = 0;
                for i in 0..buf.buffers.len() {
                    let buffer_i_len = buf.buffers[i].len().min(data_len - left);

                    buf.buffers[i][..buffer_i_len]
                        .copy_from_slice(&data[left..(left + buffer_i_len)]);

                    left += buffer_i_len;
                    if left == data_len {
                        break;
                    }
                }
                return Ok(left);
            } else {
                let current = current_task().unwrap();
                mutex_socket.block_task = Some(current);
                drop(mutex_socket);
                block_current_and_run_next();
            }
        }
    }

    fn write(&self, buf: crate::mm::UserBuffer) -> Result<usize, isize> {
        let mut data = vec![0u8; buf.len()];
        let mut left = 0;
        for i in 0..buf.buffers.len() {
            data[left..(left + buf.buffers[i].len())].copy_from_slice(buf.buffers[i]);
            left += buf.buffers[i].len();
        }

        let mut count = data.len();
        debug!("socket send len: {}", count);
        // get sock and sequence
        let (seq_number, ack_number) = get_s_a_by_index(self.socket_index)
            .map_or((TcpSeqNumber::default(), None), |x| x);
        let tcp_repr = build_tcp_repr(
            self.dst_port, 
            self.src_port, 
            TcpControl::Psh, 
            seq_number, 
            ack_number, 
            data.as_ref()
        );
        let src_ip = self.dst_ip;
        let dst_ip = self.src_ip;
        let ipv4_repr = build_ipv4_repr(src_ip, dst_ip, IpProtocol::Tcp, tcp_repr.buffer_len());
        let eth_repr = build_eth_repr(NET_STACK.mac_addr, self.src_mac, EthernetProtocol::Ipv4);
        if let Some(eth_frame) = build_eth_frame(eth_repr, None, Some((ipv4_repr, tcp_repr))) {
            NetDevice.transmit(&eth_frame.into_inner());
        }
        log::trace!("write tcp socket ok");
        Ok(count)
    }
}

impl Drop for TCP {
    fn drop(&mut self) {
        remove_socket(self.socket_index)
    }
}


async fn async_read(socket_index: usize, mut buf: crate::mm::UserBuffer, cid: usize, pid: usize) {
    let mut helper = Box::new(ReadHelper::new());
    let socket = get_mutex_socket(socket_index).unwrap();
    // info!("async read!: {}", socket_index);
    loop {
        let mut mutex_socket = socket.lock();
        // info!("async get lock!: {}", socket_index);
        if let Some(data) = mutex_socket.buffers.pop_front() {
            drop(mutex_socket);
            let data_len = data.len();
            let mut left = 0;
            for i in 0..buf.buffers.len() {
                let buffer_i_len = buf.buffers[i].len().min(data_len - left);

                buf.buffers[i][..buffer_i_len]
                    .copy_from_slice(&data[left..(left + buffer_i_len)]);

                left += buffer_i_len;
                if left == data_len {
                    break;
                }
            }
            break;
        } else {
            // info!("suspend current coroutine!: {}", socket_index);
            ASYNC_RDMP.lock().insert(socket_index, lib_so::current_cid(true));
            drop(mutex_socket);
            // suspend_current_and_run_next();
            helper.as_mut().await;
        }
    }
    // info!("wake: {}", cid);
    
    let _ = push_message(pid, UserTrapRecord {
        cause: 1,
        message: cid,
    });
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