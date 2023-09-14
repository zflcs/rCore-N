
use alloc::boxed::Box;
use alloc::vec;
use executor::Coroutine;
use lose_net_stack::packets::tcp::TCPPacket;
use lose_net_stack::IPv4;
use lose_net_stack::MacAddress;
use lose_net_stack::TcpFlags;
use ubuf::UserBuffer;
use vfs::File;
use crate::arch::uintr::uirs_send;
use crate::driver::net::NetDevice;
use crate::task::Scheduler;
use crate::task::TASK_MANAGER;
use crate::task::cpu;
use crate::task::do_block;
use crate::task::KernTask;

use super::socket::get_mutex_socket;
use super::socket::{add_socket, get_s_a_by_index, remove_socket};
use super::LOSE_NET_STACK;


pub struct TCP {
    pub target: IPv4,
    pub sport: u16,
    pub dport: u16,
    pub seq: u32,
    pub ack: u32,
    pub socket_index: usize,
}

impl TCP {
    pub fn new(target: IPv4, sport: u16, dport: u16, seq: u32, ack: u32) -> Option<Self> {
        match add_socket(target, sport, dport, seq, ack) {
            Some(index) => {
                Some(
                    Self {
                        target,
                        sport,
                        dport,
                        seq,
                        ack,
                        socket_index: index,
                    }
                )
            }
            _ => {
                None
            }
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
        log::trace!("read from tcp");
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
        let lose_net_stack = LOSE_NET_STACK.0.lock();

        let mut data = vec![0u8; buf.len()];

        let mut count = 0;
        data.copy_from_slice(buf);
        count += buf.len();

        log::trace!("socket send len: {}", count);

        // get sock and sequence
        let (seq, ack) = get_s_a_by_index(self.socket_index).map_or((0, 0), |x| x);
        log::trace!("[TCP write] seq: {}, ack: {}", seq, ack);
        let tcp_packet = TCPPacket {
            source_ip: lose_net_stack.ip,
            source_mac: lose_net_stack.mac,
            source_port: self.sport,
            dest_ip: self.target,
            dest_mac: MacAddress::new([0xff, 0xff, 0xff, 0xff, 0xff, 0xff]),
            dest_port: self.dport,
            data_len: count,
            seq,
            ack,
            flags: TcpFlags::A,
            win: 65535,
            urg: 0,
            data: data.as_ref(),
        };
        NetDevice.transmit(&tcp_packet.build_data());
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