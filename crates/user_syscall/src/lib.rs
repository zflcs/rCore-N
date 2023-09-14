#![no_std]

pub mod async_help;

use core::arch::asm;

const SYSCALL_DUP: usize = 24;
const SYSCALL_OPEN: usize = 56;
const SYSCALL_CLOSE: usize = 57;
const SYSCALL_PIPE: usize = 59;
const SYSCALL_READ: usize = 63;
const SYSCALL_WRITE: usize = 64;
const SYSCALL_EXIT: usize = 93;
const SYSCALL_NANOSLEEP: usize = 101;
const SYSCALL_GET_TIME: usize = 113;
const SYSCALL_GETPID: usize = 172;
const SYSCALL_GETTID: usize = 178;
const SYSCALL_FORK: usize = 220;
const SYSCALL_EXEC: usize = 221;
const SYSCALL_WAITPID: usize = 260;
const SYSCALL_UINTR_REGISTER_RECEIVER: usize = 244;
const SYSCALL_UINTR_CREATE_FD: usize = 246;
const SYSCALL_UINTR_REGISTER_SENDER: usize = 247;
const SYSCALL_LISTEN: usize = 300;
const SYSCALL_ACCEPT: usize = 301;
const SYSCALL_UINTR_TEST: usize = 302;

fn syscall(id: usize, args: [usize; 3]) -> isize {
    let mut ret: isize;
    unsafe {
        asm!(
            "ecall",
            inlateout("x10") args[0] => ret,
            in("x11") args[1],
            in("x12") args[2],
            in("x17") id
        );
    }
    ret
}

fn syscall6(id: usize, args: [usize; 6]) -> isize {
    let mut ret: isize;
    unsafe {
        asm!(
            "ecall",
            inlateout("x10") args[0] => ret,
            in("x11") args[1],
            in("x12") args[2],
            in("x13") args[3],
            in("x14") args[4],
            in("x15") args[5],
            in("x17") id
        );
    }
    ret
}


use async_help::AsyncCall;
use bitflags::bitflags;

bitflags! {
    pub struct OpenFlags: u32 {
        const RDONLY = 0;
        const WRONLY = 1 << 0;
        const RDWR = 1 << 1;
        const CREATE = 1 << 9;
        const TRUNC = 1 << 10;
    }
}

pub fn dup(fd: usize) -> isize {
    syscall(SYSCALL_DUP, [fd, 0, 0])
}

pub fn open(path: &str, flags: OpenFlags) -> isize {
    syscall(SYSCALL_OPEN, [path.as_ptr() as usize, flags.bits as usize, 0])
}

pub fn close(fd: usize) -> isize {
    syscall(SYSCALL_CLOSE, [fd, 0, 0])
}

pub fn pipe(pipe_fd: &mut [usize]) -> isize {
    syscall(SYSCALL_PIPE, [pipe_fd.as_mut_ptr() as usize, 0, 0])
}

pub fn read(fd: usize, buf: &mut [u8]) -> isize {
    syscall6(SYSCALL_READ, [fd, buf.as_mut_ptr() as usize, buf.len(), 0, 0, 0])
}

pub fn write(fd: usize, buf: &[u8]) -> isize {
    syscall(SYSCALL_WRITE, [fd, buf.as_ptr() as usize, buf.len()])
}

pub fn exit(exit_code: i32) -> ! {
    syscall(SYSCALL_EXIT, [exit_code as usize, 0, 0]);
    panic!("sys_exit never returns!");
}

pub fn getpid() -> isize {
    syscall(SYSCALL_GETPID, [0, 0, 0])
}

pub fn fork() -> isize {
    syscall(SYSCALL_FORK, [17, 0, 0])
}

pub fn exec(path: &str, args: &[*const u8]) -> isize {
    syscall(
        SYSCALL_EXEC,
        [path.as_ptr() as usize, args.as_ptr() as usize, 0],
    )
}

pub fn wait(exit_code: &mut i32) -> isize {
    syscall(SYSCALL_WAITPID, [usize::MAX, exit_code as *mut _ as usize, 0])
}

pub fn waitpid(pid: usize, exit_code: &mut i32) -> isize {
    syscall(SYSCALL_WAITPID, [pid, exit_code as *mut _ as usize, 0])
}

pub fn waitpid_nb(pid: usize, exit_code: &mut i32) -> isize {
    syscall(SYSCALL_WAITPID, [pid, exit_code as *mut _ as usize, 0])
}

use time_subsys::TimeSpec;
pub fn sleep(sec: f64) {
    let mut req = TimeSpec::new(sec);
    let mut rem = TimeSpec::default();
    syscall(SYSCALL_NANOSLEEP, [&mut req as *mut TimeSpec as usize, &mut rem as *mut TimeSpec as usize, 0]);
}

pub fn get_time() -> f64 {
    let mut time = TimeSpec::default();
    syscall(SYSCALL_GET_TIME, [0, &mut time as *mut TimeSpec as usize, 0]);
    time.time_in_sec()
}

pub fn gettid() -> usize {
    syscall(SYSCALL_GETTID, [0, 0, 0]) as _
}

/// exist bug
pub fn thread_create() -> usize {
    syscall(SYSCALL_FORK, [256, 0, 0]) as _
}

pub fn uintr_register_receiver() -> usize {
    syscall(SYSCALL_UINTR_REGISTER_RECEIVER, [0, 0, 0]) as _
}

pub fn uintr_register_sender(fd: usize) -> isize {
    syscall(SYSCALL_UINTR_REGISTER_SENDER, [fd, 0, 0])
}

pub fn uintr_create_fd(vector: usize) -> isize {
    syscall(SYSCALL_UINTR_CREATE_FD, [vector, 0, 0])
}

pub fn listen(port: usize) -> isize {
    syscall(SYSCALL_LISTEN, [port, 0, 0])
}

pub fn accept(fd: usize) -> isize {
    syscall(SYSCALL_ACCEPT, [fd, 0, 0])
}

pub fn uintr_test(fd: usize) -> isize {
    syscall(SYSCALL_UINTR_TEST, [fd, 0, 0])
}

pub async fn aread(fd: usize, buf: &mut [u8], cid: usize) {
    syscall6(SYSCALL_READ, [fd, buf.as_mut_ptr() as usize, buf.len(), cid, 0, 0]);
    AsyncCall::new().await;
}
