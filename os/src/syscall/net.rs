use crate::{task::{current_process, current_task, current_trap_cx, block_current_and_run_next, suspend_current_and_run_next}, net::*};
use alloc::sync::Arc;
use smoltcp::time::Duration;

#[cfg(feature = "board_qemu")]
use crate::net::{accept, listen, port_acceptable, PortFd};

// listen a port
pub fn sys_listen(port: u16) -> isize {
    #[cfg(feature = "board_qemu")]
    match listen(port) {
        Some(port_index) => {
            let process = current_process().unwrap();
            let mut inner = process.acquire_inner_lock();
            let fd = inner.alloc_fd();
            let port_fd = PortFd::new(port_index);
            inner.fd_table[fd] = Some(Arc::new(port_fd));

            // NOTICE: this return the port index, not the fd
            port_index as isize
        }
        None => -1,
    }
    #[cfg(feature = "board_axu15eg")]
    {
        use smoltcp::socket::tcp::{Socket, SocketBuffer, State};
        use alloc::vec;
        use crate::net::{SOCKET_SET, TcpFile};
        let tcp_rx_buffer = SocketBuffer::new(vec![0; 15000]);
        let tcp_tx_buffer = SocketBuffer::new(vec![0; 15000]);
        let mut tcp_socket = Socket::new(tcp_rx_buffer, tcp_tx_buffer);
        // tcp_socket.set_ack_delay(Some(Duration::from_millis(5)));
        tcp_socket.set_ack_delay(None);
        tcp_socket.set_nagle_enabled(false);
        tcp_socket.listen(port).unwrap();
        let tcp_handle = SOCKET_SET.lock().add(tcp_socket);
        loop {
            let mut binding = SOCKET_SET.lock();
            let socket = binding.get_mut::<Socket>(tcp_handle);
            if socket.is_active() {
                drop(socket);
                drop(binding);
                break;
            } else {
                drop(socket);
                drop(binding);
                suspend_current_and_run_next();
            }  
        }
        let process = current_process().unwrap();
        let mut inner = process.acquire_inner_lock();
        let fd = inner.alloc_fd();
        let tcp_file = TcpFile::new(tcp_handle);
        inner.fd_table[fd] = Some(Arc::new(tcp_file));
        fd as _
    }
}

// accept a tcp connection
pub fn sys_accept(port_index: usize) -> isize {
    #[cfg(feature = "board_qemu")]
    {
        debug!("accepting port {}", port_index);
        let task = current_task().unwrap();
        accept(port_index, task);
        // suspend_current_and_run_next();
        block_current_and_run_next();
        // net_interrupt_handler();
        // NOTICE: There does not have interrupt handler, just call it munually.
        loop {
            if !port_acceptable(port_index) {
                break;
            }
        }
        debug!("recived!!!!");
        let cx = current_trap_cx();
        cx.x[10] as isize
    }
    #[cfg(feature = "board_axu15eg")]
    0
}
