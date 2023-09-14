use crate::task::{Task, TASK_MANAGER, Scheduler, TaskState};
use alloc::{sync::Arc, vec::Vec};
use spin::Mutex;
use lazy_static::lazy_static;
use lose_net_stack::packets::tcp::TCPPacket;
use vfs::File;
use super::tcp::TCP;
pub struct Port {
    pub port: u16,
    pub receivable: bool,
    pub schedule: Option<Arc<Task>>,
}

lazy_static! {
    static ref LISTEN_TABLE: Mutex<Vec<Option<Port>>> = Mutex::new(Vec::new());
}

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
pub fn accept(listen_index: usize, task: Arc<Task>) {
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
pub fn check_accept(port: u16, tcp_packet: &TCPPacket) -> Option<()> {
    let mut listen_table = LISTEN_TABLE.lock();
    let mut listen_ports: Vec<&mut Option<Port>> = listen_table
            .iter_mut()
            .filter(|x| match x {
                Some(t) => t.port == port && t.receivable == true,
                None => false,
            })
            .collect();
    if listen_ports.len() == 0 {
        log::trace!("no listen");
        None
    } else {
        let listen_port = listen_ports[0].as_mut().unwrap();
        let task = listen_port.schedule.clone().unwrap();
        
        if accept_connection(port, tcp_packet, task) {
            listen_port.receivable = false;
            let task = listen_port.schedule.take().unwrap();
            task.locked_inner().state = TaskState::RUNNABLE;
            let _ = TASK_MANAGER.lock().add(crate::task::KernTask::Proc(task));
            Some(())
        } else {
            None
        }
    }
}

pub fn accept_connection(_port: u16, tcp_packet: &TCPPacket, task: Arc<Task>) -> bool {
    match TCP::new(
        tcp_packet.source_ip,
        tcp_packet.dest_port,
        tcp_packet.source_port,
        0,
        tcp_packet.seq + 1,
    ) {
        Some(tcp_socket) => {
            let fd = task.files().push(Arc::new(tcp_socket)).unwrap();
            log::trace!("[accept_connection]: local fd: {}, sport: {}, dport: {}", fd, tcp_packet.dest_port, tcp_packet.source_port);
            task.trapframe().set_a0(fd);
            true
        }
        _ => {
            log::trace!("invaild accept req");
            false
        }
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
}