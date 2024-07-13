use super::File;
use crate::{
    fs::ReadHelper,
    mm::UserBuffer,
    syscall::{AsyncKey, WRMAP},
    task::suspend_current_and_run_next,
    trap::{push_trap_record, UserTrapRecord},
};
use alloc::{
    boxed::Box,
    sync::{Arc, Weak},
};
use core::{future::Future, pin::Pin};
use spin::Mutex;

#[derive(Clone)]
pub struct Pipe {
    readable: bool,
    writable: bool,
    buffer: Arc<Mutex<PipeRingBuffer>>,
}

impl Pipe {
    /// 只读管道
    pub fn read_end_with_buffer(buffer: Arc<Mutex<PipeRingBuffer>>) -> Self {
        Self {
            readable: true,
            writable: false,
            buffer,
        }
    }

    /// 只写管道
    pub fn write_end_with_buffer(buffer: Arc<Mutex<PipeRingBuffer>>) -> Self {
        Self {
            readable: false,
            writable: true,
            buffer,
        }
    }
}

const RING_BUFFER_SIZE: usize = 4096;

#[derive(Copy, Clone, PartialEq)]
enum RingBufferStatus {
    FULL,
    EMPTY,
    NORMAL,
}

/// 可读/可写的环形缓冲区（每次只允许读写一个字节）
pub struct PipeRingBuffer {
    arr: [u8; RING_BUFFER_SIZE],
    head: usize,
    tail: usize,
    status: RingBufferStatus,
    write_end: Option<Weak<Pipe>>,
    read_end: Option<Weak<Pipe>>,
}

impl PipeRingBuffer {
    /// 已初始化的空缓冲区；无读写端
    pub fn new() -> Self {
        Self {
            arr: [0; RING_BUFFER_SIZE],
            head: 0,
            tail: 0,
            status: RingBufferStatus::EMPTY,
            write_end: None,
            read_end: None,
        }
    }

    /// 放置写端
    pub fn set_write_end(&mut self, write_end: &Arc<Pipe>) {
        self.write_end = Some(Arc::downgrade(write_end));
    }

    /// 放置读端
    pub fn set_read_end(&mut self, read_end: &Arc<Pipe>) {
        self.read_end = Some(Arc::downgrade(read_end))
    }

    /// 写入一个字节
    pub fn write_byte(&mut self, byte: u8) {
        self.status = RingBufferStatus::NORMAL;
        self.arr[self.tail] = byte;
        self.tail = (self.tail + 1) % RING_BUFFER_SIZE;
        if self.tail == self.head {
            self.status = RingBufferStatus::FULL;
        }
    }

    /// 读取一个字节
    pub fn read_byte(&mut self) -> u8 {
        self.status = RingBufferStatus::NORMAL;
        let c = self.arr[self.head];
        self.head = (self.head + 1) % RING_BUFFER_SIZE;
        if self.head == self.tail {
            self.status = RingBufferStatus::EMPTY;
        }
        c
    }

    /// 可读字节容量
    pub fn available_read(&self) -> usize {
        if self.status == RingBufferStatus::EMPTY {
            0
        } else if self.tail > self.head {
            self.tail - self.head
        } else {
            self.tail + RING_BUFFER_SIZE - self.head
        }
    }

    /// 可写字节容量
    pub fn available_write(&self) -> usize {
        if self.status == RingBufferStatus::FULL {
            0
        } else {
            RING_BUFFER_SIZE - self.available_read()
        }
    }

    /// 写端是否关闭
    pub fn all_write_ends_closed(&self) -> bool {
        // 未放置写端则 panic
        self.write_end.as_ref().unwrap().upgrade().is_none()
    }

    /// 读端是否关闭
    pub fn all_read_ends_closed(&self) -> bool {
        // 未放置读端则 panic
        self.read_end.as_ref().unwrap().upgrade().is_none()
    }
}

/// Return (read_end, write_end)
pub fn make_pipe() -> (Arc<Pipe>, Arc<Pipe>) {
    let buffer = Arc::new(Mutex::new(PipeRingBuffer::new()));
    let read_end = Arc::new(Pipe::read_end_with_buffer(buffer.clone()));
    let write_end = Arc::new(Pipe::write_end_with_buffer(buffer.clone()));
    buffer.lock().set_write_end(&write_end);
    buffer.lock().set_read_end(&read_end);
    (read_end, write_end)
}

impl File for Pipe {
    fn read(&self, buf: UserBuffer) -> Result<usize, isize> {
        assert!(self.readable);
        let mut buf_iter = buf.into_iter();
        let mut read_size = 0usize;
        loop {
            let mut ring_buffer = self.buffer.lock();
            let loop_read = ring_buffer.available_read();
            if loop_read == 0 {
                if ring_buffer.all_write_ends_closed() {
                    return Ok(read_size);
                }
                drop(ring_buffer);
                // debug!("[pipe sync read] suspend");
                suspend_current_and_run_next();
                continue;
            }
            // read at most loop_read bytes
            for _ in 0..loop_read {
                if let Some(byte_ref) = buf_iter.next() {
                    unsafe {
                        *byte_ref = ring_buffer.read_byte();
                    }
                    read_size += 1;
                } else {
                    return Ok(read_size);
                }
            }

            if buf_iter.is_full() {
                return Ok(read_size);
            }
        }
    }
    fn write(&self, buf: UserBuffer) -> Result<usize, isize> {
        assert!(self.writable);
        let mut buf_iter = buf.into_iter();
        let mut write_size = 0usize;
        loop {
            let mut ring_buffer = self.buffer.lock();
            let loop_write = ring_buffer.available_write();
            if loop_write == 0 {
                debug!("iter ++");
                if ring_buffer.all_read_ends_closed() {
                    debug!("pipe readFD closed");
                    return Ok(write_size);
                }
                drop(ring_buffer);
                suspend_current_and_run_next();
                continue;
            }
            // write at most loop_write bytes
            for _ in 0..loop_write {
                if let Some(byte_ref) = buf_iter.next() {
                    ring_buffer.write_byte(unsafe { *byte_ref });
                    write_size += 1;
                } else {
                    debug!("pipe write end");
                    return Ok(write_size);
                }
            }
        }
    }
    fn awrite(
        &self,
        buf: UserBuffer,
        pid: usize,
        key: usize,
    ) -> Pin<Box<dyn Future<Output = ()> + 'static + Send + Sync>> {
        Box::pin(awrite_work(self.clone(), buf, pid, key))
    }
    fn aread(
        &self,
        buf: UserBuffer,
        cid: usize,
        pid: usize,
        key: usize,
    ) -> Pin<Box<dyn Future<Output = ()> + 'static + Send + Sync>> {
        Box::pin(aread_work(self.clone(), buf, cid, pid, key))
    }

    fn readable(&self) -> bool {
        self.readable
    }

    fn writable(&self) -> bool {
        self.writable
    }
}

async fn awrite_work(s: Pipe, buf: UserBuffer, pid: usize, key: usize) {
    assert!(s.writable);
    let mut buf_iter = buf.into_iter();
    let mut write_size = 0usize;
    let mut helper = Box::new(ReadHelper::new());
    loop {
        let mut ring_buffer = s.buffer.lock();
        let loop_write = ring_buffer.available_write();
        if loop_write == 0 {
            debug!("iter ++");
            if ring_buffer.all_read_ends_closed() {
                debug!("pipe readFD closed");
                break;
            }
            drop(ring_buffer);
            // suspend_current_and_run_next();
            helper.as_mut().await;
            continue;
        }
        // write at most loop_write bytes
        for _ in 0..loop_write {
            if let Some(byte_ref) = buf_iter.next() {
                ring_buffer.write_byte(unsafe { *byte_ref });
                write_size += 1;
            } else {
                break;
            }
        }
        if buf_iter.is_full() {
            debug!("write complete!");
            break;
        }
    }
    let async_key = AsyncKey { pid, key };
    // 向文件中写完数据之后，需要唤醒内核当中的协程，将管道中的数据写到缓冲区中
    if let Some(kernel_cid) = WRMAP.lock().remove(&async_key) {
        // info!("kernel_cid {}", kernel_cid);
        lib_so::re_back(kernel_cid, 0);
    }
    debug!("pipe write end write_size={write_size}");
}

async fn aread_work(s: Pipe, buf: UserBuffer, cid: usize, pid: usize, key: usize) {
    let mut buf_iter = buf.into_iter();
    // let mut read_size = 0usize;
    let mut helper = Box::new(ReadHelper::new());
    loop {
        let mut ring_buffer = s.buffer.lock();
        let loop_read = ring_buffer.available_read();
        if loop_read == 0 {
            debug!("read_size is 0");
            if ring_buffer.all_write_ends_closed() {
                break;
                //return read_size;
            }
            drop(ring_buffer);
            WRMAP
                .lock()
                .insert(AsyncKey { pid, key }, lib_so::current_cid(true));
            helper.as_mut().await;
            continue;
        }
        debug!("read_size is {}", loop_read);
        // read at most loop_read bytes
        for _ in 0..loop_read {
            if let Some(byte_ref) = buf_iter.next() {
                unsafe {
                    *byte_ref = ring_buffer.read_byte();
                }
            } else {
                break;
            }
        }
        if buf_iter.is_full() {
            debug!("read complete!");
            break;
        }
    }
    // 将读协程加入到回调队列中，使得用户态的协程执行器能够唤醒读协程
    let res = push_trap_record(
        pid,
        UserTrapRecord {
            cause: 1,
            message: cid,
        },
    );
    debug!("read pid={pid} key={key} res={:?}", res);
}
