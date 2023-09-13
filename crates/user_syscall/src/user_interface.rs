use bitflags::bitflags;
use crate::*;

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
    sys_dup(fd)
}
pub fn open(path: &str, flags: OpenFlags) -> isize {
    sys_open(path, flags.bits)
}
pub fn close(fd: usize) -> isize {
    sys_close(fd)
}
pub fn pipe(pipe_fd: &mut [usize]) -> isize {
    sys_pipe(pipe_fd)
}
pub fn read(fd: usize, buf: &mut [u8]) -> isize {
    sys_read(fd, buf)
}
pub fn write(fd: usize, buf: &[u8]) -> isize {
    sys_write(fd, buf)
}
pub fn exit(exit_code: i32) -> ! {
    sys_exit(exit_code);
}
pub fn get_time() -> isize {
    sys_get_time()
}
pub fn getpid() -> isize {
    sys_getpid()
}
pub fn fork(flag: usize) -> isize {
    sys_fork(flag)
}
pub fn exec(path: &str, args: &[*const u8]) -> isize {
    sys_exec(path, args)
}
pub fn wait(exit_code: &mut i32) -> isize {
    sys_waitpid(-1, exit_code as *mut _)
}

pub fn waitpid(pid: usize, exit_code: &mut i32) -> isize {
    sys_waitpid(pid as isize, exit_code as *mut _)
}

pub fn waitpid_nb(pid: usize, exit_code: &mut i32) -> isize {
    sys_waitpid(pid as isize, exit_code as *mut _)
}


pub fn sleep(sec: f64) {
    let mut req = TimeSpec::new(sec);
    let mut rem = TimeSpec::default();
    sys_nanosleep(&mut req, &mut rem);
}

pub fn gettid() -> usize {
    sys_gettid() as usize
}

pub fn thread_create() -> usize {
    sys_thread_create() as usize
}



pub fn uintr_register_receiver() -> usize {
    sys_uintr_register_receiver() as usize
}

pub fn uintr_register_sender(fd: usize) -> isize {
    sys_uintr_register_sender(fd)
}

pub fn uintr_create_fd(vector: usize) -> isize {
    sys_uintr_create_fd(vector)
}

pub fn listen(port: usize) -> isize {
    sys_listen(port)
}

pub fn accept(fd: usize) -> isize {
    sys_accept(fd)
}

pub fn uintr_test(fd: usize) -> isize {
    sys_uintr_test(fd)
}