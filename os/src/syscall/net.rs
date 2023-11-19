// use crate::net::{TcpFile, SOCKET_SET};
// use crate::task::{current_process, suspend_current_and_run_next};
// use alloc::sync::Arc;
// use alloc::vec;
// use smoltcp::socket::tcp::{Socket, SocketBuffer};

// // listen a port
// pub fn sys_listen(port: u16) -> isize {
//     let tcp_rx_buffer = SocketBuffer::new(vec![0; 15000]);
//     let tcp_tx_buffer = SocketBuffer::new(vec![0; 15000]);
//     let mut tcp_socket = Socket::new(tcp_rx_buffer, tcp_tx_buffer);
//     // tcp_socket.set_ack_delay(Some(Duration::from_millis(5)));
//     tcp_socket.set_ack_delay(None);
//     tcp_socket.set_nagle_enabled(false);
//     tcp_socket.listen(port).unwrap();
//     let tcp_handle = SOCKET_SET.lock().add(tcp_socket);
//     loop {
//         let mut binding = SOCKET_SET.lock();
//         let socket = binding.get_mut::<Socket>(tcp_handle);
//         if socket.is_active() {
//             drop(binding);
//             break;
//         } else {
//             drop(binding);
//             suspend_current_and_run_next();
//         }
//     }
//     let process = current_process().unwrap();
//     let mut inner = process.acquire_inner_lock();
//     let fd = inner.alloc_fd();
//     let tcp_file = TcpFile::new(tcp_handle);
//     inner.fd_table[fd] = Some(Arc::new(tcp_file));
//     fd as _
// }
