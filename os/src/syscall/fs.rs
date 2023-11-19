// use crate::fs::make_pipe;
use crate::mm::{translated_byte_buffer, translated_refmut, UserBuffer};
use crate::task::{current_process, current_user_token};
use alloc::{collections::BTreeMap, sync::Arc};
use spin::{Lazy, Mutex};

#[derive(Debug, Clone, Copy, PartialOrd, PartialEq, Ord, Eq)]
pub struct AsyncKey {
    pub pid: usize,
    pub key: usize,
}

// key -> r_id, write coroutine can use WRMAP to find the corresponding read coroutine id
pub static WRMAP: Lazy<Arc<Mutex<BTreeMap<AsyncKey, usize>>>> =
    Lazy::new(|| Arc::new(Mutex::new(BTreeMap::new())));

pub fn sys_write(fd: usize, buf: *const u8, len: usize, key: usize, pid: usize) -> isize {
    if fd == 3 || fd == 4 || fd == 0 || fd == 1 {
        // debug!("sys_write {} {}", fd, len);
    }
    // debug!("buffer len: {}", len);
    let token = current_user_token();
    let process = current_process().unwrap();
    let inner = process.acquire_inner_lock();
    if fd >= inner.fd_table.len() {
        return -1;
    }
    if let Some(file) = &inner.fd_table[fd] {
        let file = file.clone();
        // release Task lock manually to avoid deadlock
        drop(inner);
        if key == usize::MAX {
            if let Ok(buffers) = translated_byte_buffer(token, buf, len) {
                match file.write(UserBuffer::new(buffers)) {
                    Ok(write_len) => write_len as isize,
                    Err(_) => -2,
                }
            } else {
                -3
            }
        } else {
            if let Ok(count) = file.awrite(
                UserBuffer::new(translated_byte_buffer(token, buf, len).unwrap()),
                pid,
                key,
            ) {
                count as _
            } else {
                -2
            }
        }
    } else {
        -4
    }
}

pub fn sys_read(fd: usize, buf: *const u8, len: usize, key: usize, cid: usize) -> isize {
    if fd == 3 || fd == 4 || fd == 0 || fd == 1 {
        // debug!("sys_read {} {}", fd, len);
    }
    let token = current_user_token();
    let process = current_process().unwrap();
    let pid = process.pid.0;
    let inner = process.acquire_inner_lock();
    // info!("test1: {}", fd);
    if fd >= inner.fd_table.len() {
        return -1;
    }
    if let Some(file) = &inner.fd_table[fd] {
        let file = file.clone();
        // release Task lock manually to avoid deadlock
        drop(inner);
        if key == usize::MAX && cid == usize::MAX {
            if let Ok(buffers) = translated_byte_buffer(token, buf, len) {
                match file.read(UserBuffer::new(buffers)) {
                    Ok(read_len) => read_len as isize,
                    Err(_) => -2,
                }
            } else {
                -3
            }
        } else {
            if let Ok(count) = file.aread(
                UserBuffer::new(translated_byte_buffer(token, buf, len).unwrap()),
                cid,
                pid,
                key,
            ) {
                count as _
            } else {
                -2
            }
        }
    } else {
        -4
    }
}

pub fn sys_close(fd: usize) -> isize {
    let task = current_process().unwrap();
    let mut inner = task.acquire_inner_lock();
    if fd >= inner.fd_table.len() {
        return -1;
    }
    if inner.fd_table[fd].is_none() {
        return -1;
    }
    inner.fd_table[fd].take();
    0
}

// pub fn sys_pipe(pipe: *mut usize) -> isize {
//     let task = current_process().unwrap();
//     let token = current_user_token();
//     let mut inner = task.acquire_inner_lock();
//     let (pipe_read, pipe_write) = make_pipe();
//     let read_fd = inner.alloc_fd();
//     inner.fd_table[read_fd] = Some(pipe_read);
//     let write_fd = inner.alloc_fd();
//     inner.fd_table[write_fd] = Some(pipe_write);
//     *translated_refmut(token, pipe) = read_fd;
//     *translated_refmut(token, unsafe { pipe.add(1) }) = write_fd;
//     0
// }
