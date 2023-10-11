
use alloc::boxed::Box;
use alloc::vec;
use smoltcp::time::Instant;
use crate::device::net::NET_DEVICE;
use crate::fs::File;
use crate::mm::UserBuffer;


use crate::syscall::sys_thread_create;
use crate::task::{block_current_and_run_next, current_task, suspend_current_and_run_next, TaskControlBlock, current_process};
use crate::trap::{UserTrapRecord, push_trap_record};
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

    fn awrite(&self, buf: crate::mm::UserBuffer, pid: usize, key: usize) -> Result<usize, isize> {
        todo!()
    }

    fn aread(&self, mut buf: crate::mm::UserBuffer, cid: usize, pid: usize, key: usize) -> Result<usize, isize> {
        #[cfg(feature = "kcuc")]
        {
            let work = Box::pin(async_read(self.0, buf, cid, pid));
            lib_so::spawn(move || work, 0, 0, lib_so::CoroutineKind::KernSyscall);
            Ok(0)
        }
        #[cfg(feature = "ktuc")]
        {
            // check current process's poll thread is or not exist
            let current_process = current_process().unwrap();
            // add socket & user coroutine relation to map
            current_process.acquire_inner_lock().get_socket2ktaskinfo().lock().push((self.0, (buf, cid, pid)));
            if !current_process.acquire_inner_lock().has_poll_thread {
                kernel_thread_create(poll_socket_thread as _);      // create a thread & add_task to scheduler
                current_process.acquire_inner_lock().has_poll_thread = true;
            }
            Ok(0)
        }
    }

    /// only two scenario will break the loop 
    /// 1. the socket cannot receive
    /// 2. the buffer is full
    fn read(&self, mut buf: UserBuffer) -> Result<usize, isize> {
        #[cfg(feature = "ktut")]
        {
            let mut buf_iter = buf.buffers.iter_mut();
            let mut head_buf = buf_iter.next();
            let mut count = 0usize;
            loop {
                iface_poll();
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
        #[cfg(feature = "kcut")]
        {
            let current_task = current_task().unwrap();
            let work = thread_async_read(
                current_task, 
                self.0, 
                buf
            );
            lib_so::spawn(move || work, 0, 0, lib_so::CoroutineKind::KernSyscall);
            block_current_and_run_next();
            Ok(0)
        }

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
        iface_poll();
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
    let _ = push_trap_record(pid, UserTrapRecord {
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

#[cfg(feature = "kcut")]
pub async fn thread_async_read(thread: Arc<TaskControlBlock>, handle: SocketHandle, mut buf: UserBuffer) {
    use crate::task::add_task;
    let mut buf_iter = buf.buffers.iter_mut();
    let mut head_buf = buf_iter.next();
    let mut count = 0usize;
    let waker = TcpSocketWaker::new(lib_so::current_cid(true));
    let mut helper = Box::new(ReadHelper::new());
    loop {
        iface_poll();
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
    // wake up the block thread
    log::debug!("wake thread");
    add_task(thread);
}


#[cfg(feature = "ktuc")]
/// poll socket & complete the data move option
/// this thread will not block until the process is end
pub fn poll_socket_thread() {
    use alloc::vec::Vec;

    use crate::task::{add_user_intr_task, exit_current_and_run_next, TaskStatus, schedule, take_current_task};
    loop {
        log::trace!("kernel poll_socket_thread loop");
        iface_poll();
        if let Some(curr_process) = current_process() {
            let mut process_inner = curr_process.acquire_inner_lock();
            let socket2ktaskinfo = process_inner.get_socket2ktaskinfo();
            let mut need_suspend = false;
            let mut socket2ktaskinfo_inner = socket2ktaskinfo.lock();
            let len = socket2ktaskinfo_inner.len();
            for i in 0..len {
                let (handle, mut task_info) = socket2ktaskinfo_inner.pop().unwrap();
                let mut buf_iter = task_info.0.buffers.iter_mut();
                let mut head_buf = buf_iter.next();
                let mut count = 0usize;
                loop {
                    if let Some(mut socket_set) = SOCKET_SET.try_lock() {
                        let socket = socket_set.get_mut::<Socket>(handle);
                        if socket.is_active() {
                            if head_buf.is_some() {
                                if socket.can_recv() {
                                    if let Ok(size) = socket.recv_slice(head_buf.as_mut().unwrap()) {
                                        count += size;
                                        drop(socket);
                                        drop(socket_set);
                                        head_buf = buf_iter.next();
                                    } else {
                                    }
                                } else {  // socket has no buffer, cannot receive
                                    break;
                                }
                            } else {    // buffer is full
                                break;
                            }
                        } else {    // socket is not active
                            break;
                        }
                    } else {    // cannot get socket_set
                        break;
                    }
                }
                if count > 0 {      // read ok
                    need_suspend = true;
                    // wake up user coroutine
                    log::trace!("push_trap_record");
                    add_user_intr_task(task_info.2);
                    process_inner.push_user_trap_record(UserTrapRecord {
                        cause: 1,
                        message: task_info.1,
                    });
                } else {
                    socket2ktaskinfo_inner.push((handle, task_info));
                }
            }
            drop(socket2ktaskinfo_inner);
            drop(socket2ktaskinfo);
            drop(process_inner);
            drop(curr_process);
            if need_suspend {
                log::trace!("suspend");
                suspend_current_and_run_next();
            }
        } else {
            // must take the current task from processor
            let task = take_current_task().unwrap();    
            let mut task_inner = task.acquire_inner_lock();
            // Change status to Zombie
            task_inner.task_status = TaskStatus::Zombie;
            // Record exit code
            task_inner.exit_code = Some(0);
            // warn!("exit start: {} 2", tid);
            task_inner.res = None;
            let task_cx_ptr = task_inner.get_task_cx_ptr();
            drop(task_inner);
            let mut _unused = Default::default();
            schedule(&mut _unused as *mut _);
        }
        
    }
}

#[cfg(feature = "ktuc")]
pub fn kernel_thread_create(entry: usize) -> isize {
    use crate::task::add_task;

    let task = current_task().unwrap();

    let process = task.process.upgrade().unwrap();
    // create a new thread
    let new_task = Arc::new(TaskControlBlock::new(
        Arc::clone(&process),
        task.acquire_inner_lock()
            .res
            .as_ref()
            .unwrap()
            .ustack_base,
        true,
    ));
    // debug!("tid: {}", new_task.acquire_inner_lock().res.as_ref().unwrap().tid);
    let mut new_task_inner = new_task.acquire_inner_lock();
    (unsafe { &mut *new_task_inner.get_task_cx_ptr() }).ra = entry;
    let new_task_res = new_task_inner.res.as_ref().unwrap();
    let new_task_tid = new_task_res.tid;
    let mut process_inner = process.acquire_inner_lock();
    // add new thread to current process
    let tasks = &mut process_inner.tasks;
    while tasks.len() < new_task_tid + 1 {
        tasks.push(None);
    }
    tasks[new_task_tid] = Some(Arc::clone(&new_task));

    // add new task to scheduler
    add_task(Arc::clone(&new_task));
    debug!("kernel thread create start end");
    new_task_tid as isize
}