use alloc::{sync::Arc, vec::Vec};
use smoltcp::wire::{EthernetAddress, Ipv4Address, TcpSeqNumber, TcpPacket};
use spin::Lazy;
use crate::{task::{TaskControlBlock, add_task}, fs::File};

use super::tcp::TCP;
use kernel_sync::SpinLock;


pub struct Port {
    pub port: u16,
    pub receivable: bool,
    pub schedule: Option<Arc<TaskControlBlock>>,
}

pub static LISTEN_TABLE: Lazy<SpinLock<Vec<Option<Port>>>> = Lazy::new(|| SpinLock::new(Vec::new()));

pub fn listen(port: u16) -> Option<usize> {
    let mut listen_table = LISTEN_TABLE.lock();
    let mut index = usize::MAX;
    for i in 0..listen_table.len() {
        if listen_table[i].is_none() {
            index = i;
            break;
        }
    }

    let listen_port = Port {
        port,
        receivable: false,
        schedule: None,
    };

    if index == usize::MAX {
        listen_table.push(Some(listen_port));
        Some(listen_table.len() - 1)
    } else {
        listen_table[index] = Some(listen_port);
        Some(index)
    }
}

// can accept request
pub fn accept(listen_index: usize, task: Arc<TaskControlBlock>) {
    let mut listen_table = LISTEN_TABLE.lock();
    assert!(listen_index < listen_table.len());
    let listen_port = listen_table[listen_index].as_mut();
    assert!(listen_port.is_some());
    let listen_port = listen_port.unwrap();
    listen_port.receivable = true;
    listen_port.schedule = Some(task);
}

pub fn port_acceptable(listen_index: usize) -> bool {
    let mut listen_table = LISTEN_TABLE.lock();
    assert!(listen_index < listen_table.len());

    let listen_port = listen_table[listen_index].as_mut();
    listen_port.map_or(false, |x| x.receivable)
}

// check whether it can accept request
pub fn check_accept(src_mac: EthernetAddress, src_ip: Ipv4Address, dst_ip: Ipv4Address, tcp_packet: &TcpPacket<&[u8]>) -> bool {
    let mut listen_table = LISTEN_TABLE.lock();
    let mut listen_ports: Vec<&mut Option<Port>> = listen_table
            .iter_mut()
            .filter(|x| match x {
                Some(t) => t.port == tcp_packet.dst_port() && t.receivable == true,
                None => false,
            })
            .collect();
    if listen_ports.len() == 0 {
        log::warn!("no listen");
        false
    } else {
        let listen_port = listen_ports[0].as_mut().unwrap();
        let task = listen_port.schedule.clone().unwrap();
        if accept_connection(src_mac, src_ip, dst_ip, tcp_packet, task) {
            listen_port.receivable = false;
            add_task(listen_port.schedule.take().unwrap());
            true
        } else {
            false
        }
    }
}

pub fn accept_connection(src_mac: EthernetAddress, src_ip: Ipv4Address, dst_ip: Ipv4Address, tcp_packet: &TcpPacket<&[u8]>, task: Arc<TaskControlBlock>) -> bool {
    if let Some(tcp) = TCP::new(
        src_mac,
        src_ip, 
        dst_ip, 
        tcp_packet.src_port(), 
        tcp_packet.dst_port(),
        TcpSeqNumber::default(),
        Some(tcp_packet.seq_number() + tcp_packet.segment_len()),
    ) {
        let process = task.process.upgrade().unwrap();
        let mut inner = process.acquire_inner_lock();
        let fd = inner.alloc_fd();
        log::trace!("[accept_connection]: src_ip: {:?}, src_port: {}, dst_port: {}", tcp.src_ip, tcp.src_port, tcp.dst_port);
        inner.fd_table[fd] = Some(Arc::new(tcp));
        let cx = task.acquire_inner_lock().get_trap_cx();
        cx.x[10] = fd;
        true
    } else {
        log::warn!("invaild accept req");
        false
    }
}


// store in the fd_table, delete the listen table when close the application.
pub struct PortFd(usize);

impl PortFd {
    pub fn new(port_index: usize) -> Self {
        PortFd(port_index)
    }
}

impl Drop for PortFd {
    fn drop(&mut self) {
        LISTEN_TABLE.lock()[self.0] = None
    }
}

impl File for PortFd {
    fn readable(&self) -> bool {
        false
    }

    fn writable(&self) -> bool {
        false
    }

    fn read(&self, _buf: crate::mm::UserBuffer) -> Result<usize, isize> {
        Ok(0)
    }

    fn write(&self, _buf: crate::mm::UserBuffer) -> Result<usize, isize> {
        Ok(0)
    }

    fn awrite(&self, buf: crate::mm::UserBuffer, pid: usize, key: usize) -> core::pin::Pin<alloc::boxed::Box<dyn core::future::Future<Output = ()> + 'static + Send + Sync>> {
        todo!()
    }

    fn aread(&self, buf: crate::mm::UserBuffer, cid: usize, pid: usize, key: usize) -> core::pin::Pin<alloc::boxed::Box<dyn core::future::Future<Output = ()> + 'static + Send + Sync>> {
        todo!()
    }
}