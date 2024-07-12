use super::{
    socket::{add_socket, get_mutex_socket, get_s_a_by_index, remove_socket},
    LOSE_NET_STACK,
};
use crate::{
    device::NetDevice,
    fs::{File, ReadHelper},
    net::ASYNC_RDMP,
    task::{block_current_and_run_next, current_task},
    trap::{push_trap_record, UserTrapRecord},
};
use alloc::{boxed::Box, vec};
use lose_net_stack::{packets::tcp::TCPPacket, IPv4, MacAddress, TcpFlags};

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
        add_socket(target, sport, dport, seq, ack).map(|socket_index| Self {
            target,
            sport,
            dport,
            seq,
            ack,
            socket_index,
        })
    }
}

impl File for TCP {
    fn readable(&self) -> bool {
        true
    }

    fn writable(&self) -> bool {
        true
    }

    fn read(&self, mut buf: crate::mm::UserBuffer) -> Result<usize, isize> {
        let socket = get_mutex_socket(self.socket_index).unwrap();
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
        let lose_net_stack = LOSE_NET_STACK.0.lock();

        let mut data = vec![0u8; buf.len()];

        let mut left = 0;
        for i in 0..buf.buffers.len() {
            data[left..(left + buf.buffers[i].len())].copy_from_slice(buf.buffers[i]);
            left += buf.buffers[i].len();
        }

        let len = data.len();
        debug!("socket send len: {}", len);

        // get sock and sequence
        let (seq, ack) = get_s_a_by_index(self.socket_index).map_or((0, 0), |x| x);
        debug!("[TCP write] seq: {}, ack: {}", seq, ack);
        let tcp_packet = TCPPacket {
            source_ip: lose_net_stack.ip,
            source_mac: lose_net_stack.mac,
            source_port: self.sport,
            dest_ip: self.target,
            dest_mac: MacAddress::new([0xff, 0xff, 0xff, 0xff, 0xff, 0xff]),
            dest_port: self.dport,
            data_len: len,
            seq,
            ack,
            flags: TcpFlags::A,
            win: 65535,
            urg: 0,
            data: data.as_ref(),
        };
        NetDevice.transmit(&tcp_packet.build_data());
        Ok(len)
    }

    fn awrite(
        &self,
        buf: crate::mm::UserBuffer,
        pid: usize,
        key: usize,
    ) -> core::pin::Pin<
        alloc::boxed::Box<dyn core::future::Future<Output = ()> + 'static + Send + Sync>,
    > {
        todo!()
    }

    fn aread(
        &self,
        mut buf: crate::mm::UserBuffer,
        cid: usize,
        pid: usize,
        key: usize,
    ) -> core::pin::Pin<
        alloc::boxed::Box<dyn core::future::Future<Output = ()> + 'static + Send + Sync>,
    > {
        Box::pin(async_read(self.socket_index, buf, cid, pid))
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
            for buffer in &mut buf.buffers {
                let buffer_i_len = buffer.len().min(data_len - left);

                buffer[..buffer_i_len].copy_from_slice(&data[left..(left + buffer_i_len)]);

                left += buffer_i_len;
                if left == data_len {
                    break;
                }
            }
            break;
        } else {
            // info!("suspend current coroutine!: {}", socket_index);
            ASYNC_RDMP
                .lock()
                .insert(socket_index, lib_so::current_cid(true));
            drop(mutex_socket);
            // suspend_current_and_run_next();
            helper.as_mut().await;
        }
    }
    // info!("wake: {}", cid);

    let _ = push_trap_record(
        pid,
        UserTrapRecord {
            cause: 1,
            message: cid,
        },
    );
}
