
use alloc::boxed::Box;
use alloc::vec;
use smoltcp::time::Instant;
use crate::device::net::NET_DEVICE;
use crate::fs::File;
use crate::mm::UserBuffer;


use crate::task::{block_current_and_run_next, current_task, suspend_current_and_run_next};
use crate::trap::{UserTrapRecord, push_message};
use super::{ASYNC_RDMP, SOCKET_SET, iface_poll};
use super::iface::INTERFACE;
use smoltcp::socket::tcp::{Socket, SocketBuffer};
use smoltcp::iface::SocketHandle;

pub struct TcpFile(SocketHandle);

impl TcpFile {
    pub fn new(handle: SocketHandle) -> Self {
        Self(handle)
    }
}


impl File for TcpFile {
    fn readable(&self) -> bool {
        let socket_set = SOCKET_SET.lock();
        let socket = socket_set.get::<Socket>(self.0);
        socket.can_recv()
    }

    fn writable(&self) -> bool {
        let socket_set = SOCKET_SET.lock();
        let socket = socket_set.get::<Socket>(self.0);
        socket.can_send()
    }

    fn awrite(&self, buf: crate::mm::UserBuffer, pid: usize, key: usize) -> core::pin::Pin<alloc::boxed::Box<dyn core::future::Future<Output = ()> + 'static + Send + Sync>> {
        todo!()
    }

    fn aread(&self, mut buf: crate::mm::UserBuffer, cid: usize, pid: usize, key: usize) -> core::pin::Pin<alloc::boxed::Box<dyn core::future::Future<Output = ()> + 'static + Send + Sync>> {
        Box::pin(async_read(self.0, buf, cid, pid))
    }

    /// only two scenario will break the loop 
    /// 1. the socket cannot receive
    /// 2. the buffer is full
    fn read(&self, mut buf: UserBuffer) -> Result<usize, isize> {
        let mut buf_iter = buf.buffers.iter_mut();
        let mut head_buf = buf_iter.next();
        let mut count = 0usize;
        loop {
            let mut socket_set = SOCKET_SET.lock();
            let socket = socket_set.get_mut::<Socket>(self.0);
            if socket.is_active() {
                if head_buf.is_some() {
                    if socket.can_recv() {
                        if let Ok(size) = socket.recv_slice(head_buf.as_mut().unwrap()) {
                            count += size;
                            drop(socket);
                            drop(socket_set);
                            head_buf = buf_iter.next();
                        }
                    } else {  // socket has no buffer, need wait
                        drop(socket);
                        drop(socket_set);
                        suspend_current_and_run_next();
                        continue;
                    }
                } else {    // buffer is full
                    break;
                }
            } else {    // socket is not active
                break;
            }
        }
        Ok(count)
    }

    // send as much as possible
    fn write(&self, buf: UserBuffer) -> Result<usize, isize> {
        let mut buf_iter = buf.buffers.iter();
        let mut head_buf = buf_iter.next();
        let mut count = 0usize;
        loop {
            let mut socket_set = SOCKET_SET.lock();
            let socket = socket_set.get_mut::<Socket>(self.0);
            if socket.is_active() {
                if head_buf.is_some() {
                    if socket.can_send() {
                        if let Ok(size) = socket.send_slice(head_buf.as_mut().unwrap()) {
                            count += size;
                            drop(socket);
                            drop(socket_set);
                            head_buf = buf_iter.next();
                            iface_poll();
                        }
                    } else {  // socket has no space, need wait
                        drop(socket);
                        drop(socket_set);
                        suspend_current_and_run_next();
                        continue;
                    }
                } else {    // buffer is full
                    break;
                }
            } else {    // socket is not active
                break;
            }
        }
        Ok(count)
    }
}

impl Drop for TcpFile {
    fn drop(&mut self) {
        SOCKET_SET.lock().remove(self.0);
    }
}


async fn async_read(handle: SocketHandle, mut buf: UserBuffer, cid: usize, pid: usize) {
    let mut buf_iter = buf.buffers.iter_mut();
    let mut head_buf = buf_iter.next();
    let mut count = 0usize;
    let waker = TcpSocketWaker::new(lib_so::current_cid(true));
    let mut helper = Box::new(ReadHelper::new());
    loop {
        let mut socket_set = SOCKET_SET.lock();
        let socket = socket_set.get_mut::<Socket>(handle);
        if socket.is_active() {
            if head_buf.is_some() {
                if socket.can_recv() {
                    if let Ok(size) = socket.recv_slice(head_buf.as_mut().unwrap()) {
                        count += size;
                        drop(socket);
                        drop(socket_set);
                        head_buf = buf_iter.next();
                    }
                } else {  // socket has no buffer, need wait
                    // register waker
                    socket.register_recv_waker(unsafe { &Waker::from_raw(waker.clone().into()) });
                    drop(socket);
                    drop(socket_set);
                    log::trace!("register waker");
                    helper.as_mut().await;
                    log::trace!("be waked");
                }
            } else {    // buffer is full
                break;
            }
        } else {    // socket is not active
            break;
        }
    }
    log::trace!("push message");
    let _ = push_message(pid, UserTrapRecord {
        cause: 1,
        message: cid,
    });
}

use core::task::{Waker, RawWaker};
use core::{future::Future, pin::Pin, task::{Poll, Context}};
use alloc::{task::Wake, sync::Arc};


struct TcpSocketWaker(usize);

impl TcpSocketWaker {
    pub fn new(cid: usize) -> Arc<Self> {
        Arc::new(Self(cid))
    }
}

impl Wake for TcpSocketWaker {
    fn wake(self: Arc<Self>) {
        log::trace!("wake");
        lib_so::re_back(self.0, 0);
    }

    fn wake_by_ref(self: &Arc<Self>) {
        log::trace!("wake");
        lib_so::re_back(self.0, 0);
    }
}


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