use super::{SyscallNET, SyscallImpl};
use alloc::sync::Arc;
use errno::Errno;
use syscall_interface::SyscallResult;
use crate::{net::*, task::{cpu, do_block}};

impl SyscallNET for SyscallImpl {
    
    fn listen(port: u16) -> SyscallResult {
        if let Some(port_index) = listen(port) {
            let curr = cpu().curr.as_ref().unwrap();
            let port_fd = PortFd::new(port_index);
            let _ = curr.files().push(Arc::new(port_fd));
            Ok(port_index)
        } else {
            Err(Errno::EINVAL)
        }
    }

    fn accept(port_index: usize) -> SyscallResult {
        let task = cpu().curr.as_ref().unwrap();
        accept(port_index, task.clone());
        drop(task);
        unsafe { do_block(); }

        loop {
            if !port_acceptable(port_index) {
                break;
            }
        }
        let task = cpu().curr.as_ref().unwrap();
        let fd = task.trapframe().get_a0();
        log::debug!("accepting port {}", fd);
        Ok(fd)
    }
}