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

bitflags! {
    /// A bit mask that allows the caller to specify what is shared between the calling process and the child process.
    pub struct CloneFlags: u32 {
        /// Signal mask to be sent at exit.
        const CSIGNAL = 0x000000ff;
        /// Set if vm shared between processes. In particular, memory writes performed by the calling process or
        /// by the child process are also visible in the other process.
        const CLONE_VM = 0x00000100;
        /// Set if fs info shared between processes which includes the root of the filesystem,
        /// the current working directory, and the umask.
        const CLONE_FS = 0x00000200;
        /// Set if file descriptor table shared between processes
        const CLONE_FILES = 0x00000400;
        /// Set if signal handlers and blocked signals shared
        const CLONE_SIGHAND = 0x00000800;
        /// Set if a pidfd should be placed in parent
        const CLONE_PIDFD = 0x00001000;
        /// Set if we want to let tracing continue on the child too
        const CLONE_PTRACE = 0x00002000;
        /// Set if the parent wants the child to wake it up on mm_release
        const CLONE_VFORK = 0x00004000;
        /// Set if we want to have the same parent as the cloner
        const CLONE_PARENT = 0x00008000;
        /// Set if in the same thread group
        const CLONE_THREAD = 0x00010000;
        /// If set, the cloned child is started in a new mount namespace, initialized with a copy of
        /// the namespace of the parent.
        const CLONE_NEWNS = 0x00020000;
        /// If CLONE_SYSVSEM is set, then the child and the calling process share a single list of
        /// System V semaphore adjustment (semadj) values (see semop(2)).
        const CLONE_SYSVSEM = 0x00040000;
        /// If set, create a new TLS for the child
        const CLONE_SETTLS = 0x00080000;
        /// Store the child thread ID at the location pointed to by `parent_tid`.
        const CLONE_PARENT_SETTID = 0x00100000;
        /// Clear the child thread ID at the location pointed to by `child_tid` in child's memory
        /// when child exits, and do a wakeup on the futex at that address.
        const CLONE_CHILD_CLEARTID = 0x00200000;
        /// This flag is still defined, but it is usually ignored when calling clone().
        const CLONE_DETACHED = 0x00400000;
        /// A tracing process cannot force CLONE_PTRACE on this child process.
        const CLONE_UNTRACED = 0x00800000;
        /// Store the child thread ID at the location pointed to by `child_tid` in child's memory.
        const CLONE_CHILD_SETTID = 0x01000000;
        /// New cgroup namespace
        const CLONE_NEWCGROUP = 0x02000000;
        /// New utsname namespace
        const CLONE_NEWUTS = 0x04000000;
        /// New ipc namespace
        const CLONE_NEWIPC = 0x08000000;
        /// New user namespace
        const CLONE_NEWUSER	= 0x10000000;
        /// New pid namespace
        const CLONE_NEWPID = 0x20000000;
        /// New network namespace
        const CLONE_NEWNET = 0x40000000;
        /// Clone io context
        const CLONE_IO = 0x80000000;
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
    syscall6(SYSCALL_FORK, [17, 0, 0, 0, 0, 0])
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


pub fn thread_create(fn_ptr: usize) -> usize {
    let pid = syscall6(SYSCALL_FORK, 
        [
            (CloneFlags::CLONE_VM | CloneFlags::CLONE_FILES 
                | CloneFlags::CLONE_SIGHAND | CloneFlags::CLONE_VFORK
            ).bits() as usize | 17, 
            0, 0, 0, 0, 0
            ]
        );
    if pid == 0 {
        unsafe {
            let thread: fn() = core::mem::transmute(fn_ptr);
            thread();
        }
    }
    pid as _
}

pub fn thread_join(_tid: usize) {
    let mut exit_code = 0;
    wait(&mut exit_code);
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
