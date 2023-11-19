mod context;
mod manager;
mod pid;
mod pool;
mod process;
mod processor;
mod switch;
mod task;

use crate::fs::{open_file, OpenFlags};
use alloc::sync::Arc;
use alloc::vec::Vec;

use spin::{Mutex, Lazy};
use switch::__switch;

use crate::task::pid::TaskUserRes;
use crate::task::pool::remove_from_pid2process;
pub use context::TaskContext;
pub use pid::{pid_alloc, KernelStack, PidHandle};
pub use pool::{
    add_task, fetch_task, pid2process,
};
pub use process::ProcessControlBlock;
pub use processor::{
    current_process, current_task, current_trap_cx, current_trap_cx_user_va, current_user_token,
    hart_id, mmap, munmap, run_tasks, schedule, take_current_task
};
pub use task::{TaskControlBlock, TaskStatus};

pub static WAIT_LOCK: Lazy<Mutex<()>> = Lazy::new(|| Mutex::new(()));
pub static WAITTID_LOCK: Lazy<Mutex<()>> = Lazy::new(|| Mutex::new(()));

/// This function must be followed by a schedule
pub fn block_current_task() -> *mut TaskContext {
    let task = take_current_task().unwrap();
    let mut task_inner = task.acquire_inner_lock();
    task_inner.task_status = TaskStatus::Blocking;
    &mut task_inner.task_cx as *mut TaskContext
}

pub fn block_current_and_run_next() {
    let task_cx_ptr = block_current_task();
    schedule(task_cx_ptr);
}

pub fn suspend_current_and_run_next() {
    // There must be an application running.
    let task = current_task().unwrap();
    let mut task_inner = task.acquire_inner_lock();
    task_inner.time_intr_count += 1;
    let task_cx_ptr = task_inner.get_task_cx_ptr();
    drop(task_inner);
    // jump to scheduling cycle
    schedule(task_cx_ptr);
}

pub fn exit_current_and_run_next(exit_code: i32) {
    debug!("exit start");
    // ++++++ hold initproc PCB lock here
    // let mut initproc_inner = INITPROC.acquire_inner_lock();
    // take from Processor
    let task = take_current_task().unwrap();
    let process = task.process.upgrade().unwrap();
    // **** hold current PCB lock
    let wtl = WAITTID_LOCK.lock();
    let mut inner = task.acquire_inner_lock();
    let tid = inner.res.as_ref().unwrap().tid;
    // warn!("exit start: {}", tid);
    info!(
        "pid: {} tid: {} exited with code {}, time intr: {}, cycle count: {}, interrupt time: {}, user_cycle: {} us",
        task.getpid(), tid, exit_code, inner.time_intr_count, inner.total_cpu_cycle_count, inner.interrupt_time, inner.user_time_us
    );

    // Change status to Zombie
    inner.task_status = TaskStatus::Zombie;
    // Record exit code
    inner.exit_code = Some(exit_code);
    // warn!("exit start: {} 2", tid);
    inner.res = None;
    // warn!("exit start: {} 3", tid);
    drop(inner);
    drop(wtl);
    // do not move to its parent but under initproc
    if tid == 0 {
        let _wl = WAIT_LOCK.lock();
        let pid = process.getpid();
        remove_from_pid2process(pid);
        let mut process_inner = process.acquire_inner_lock();
        process_inner.is_zombie = true;
        process_inner.exit_code = exit_code;
        {
            let mut initproc_inner = INITPROC.acquire_inner_lock();

            for child in process_inner.children.iter() {
                child.acquire_inner_lock().parent = Some(Arc::downgrade(&INITPROC));
                initproc_inner.children.push(child.clone());
            }
        }

        let mut recycle_res = Vec::<TaskUserRes>::new();
        for task in process_inner.tasks.iter().filter(|t| t.is_some()) {
            let task = task.as_ref().unwrap();
            let mut task_inner = task.acquire_inner_lock();
            if let Some(res) = task_inner.res.take() {
                recycle_res.push(res);
            }
        }
        process_inner.children.clear();
        process_inner.mm.recycle_vma_all();
        process_inner.fd_table.clear();
        drop(process_inner);
        recycle_res.clear();
    }

    // **** release current PCB lock
    // drop task manually to maintain rc correctly
    drop(task);
    drop(process);
    // warn!("exit end: {}", tid);
    // we do not have to save task context
    debug!("exit end ");
    let mut _unused = Default::default();
    schedule(&mut _unused as *mut _);
}

pub static INITPROC: Lazy<Arc<ProcessControlBlock>> = Lazy::new(|| {
    let inode = open_file("hello", OpenFlags::RDONLY).unwrap();
    let v = inode.read_all();
    ProcessControlBlock::empty()
});

pub fn add_initproc() {
    let inode = open_file("hello", OpenFlags::RDONLY).unwrap();
    let v = inode.read_all();
    let _init = ProcessControlBlock::new(&v);
}
