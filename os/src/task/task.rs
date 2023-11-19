use super::KernelStack;
use super::TaskContext;
use crate::mm::PhysPageNum;
use crate::task::pid::{kstack_alloc, TaskUserRes};
use crate::task::process::ProcessControlBlock;
use crate::trap::TrapContext;
use alloc::sync::{Arc, Weak};
use spin::{Mutex, MutexGuard};
use crate::trap::trap_return;
pub struct TaskControlBlock {
    // immutable
    pub process: Weak<ProcessControlBlock>,
    pub kstack: KernelStack,
    // mutable
    pub inner: Mutex<TaskControlBlockInner>,
}

pub struct TaskControlBlockInner {
    pub res: Option<TaskUserRes>,
    pub trap_cx_ppn: PhysPageNum,
    pub task_cx: TaskContext,
    pub task_cx_ptr: usize,
    pub task_status: TaskStatus,
    pub exit_code: Option<i32>,
    pub time_intr_count: usize,
    pub total_cpu_cycle_count: usize,
    pub last_cpu_cycle: usize,
    pub interrupt_time: usize,
    pub user_time_us: usize,
    pub last_user_time_us: usize,
}

impl TaskControlBlockInner {
    pub fn get_task_cx_ptr(&mut self) -> *mut TaskContext {
        &mut self.task_cx as *mut TaskContext
    }
    #[deprecated]
    #[allow(unused)]
    pub fn get_task_cx_ptr2(&self) -> *const usize {
        &self.task_cx_ptr as *const usize
    }
    pub fn get_trap_cx(&self) -> &'static mut TrapContext {
        self.trap_cx_ppn.get_mut()
    }

    fn get_status(&self) -> TaskStatus {
        self.task_status
    }
    pub fn is_zombie(&self) -> bool {
        self.get_status() == TaskStatus::Zombie
    }

}

impl TaskControlBlock {
    pub fn acquire_inner_lock(&self) -> MutexGuard<TaskControlBlockInner> {
        self.inner.lock()
    }
    
    pub fn get_user_token(&self) -> usize {
        let process = self.process.upgrade().unwrap();
        let inner = process.acquire_inner_lock();
        inner.mm.token()
    }

    pub fn getpid(&self) -> usize {
        self.process.upgrade().unwrap().getpid()
    }

    pub fn new(
        process: Arc<ProcessControlBlock>,
        ustack_base: usize,
        alloc_user_res: bool,
    ) -> Self {
        let res = TaskUserRes::new(Arc::clone(&process), ustack_base, alloc_user_res);
        let tid = res.tid;
        let trap_cx_ppn = res.trap_cx_ppn();
        let kstack = kstack_alloc();
        let kstack_top = kstack.get_top();
        Self {
            process: Arc::downgrade(&process),
            kstack,
            inner: Mutex::new(TaskControlBlockInner {
                res: Some(res),
                trap_cx_ppn,
                task_cx: TaskContext::goto_target(trap_return as _, kstack_top, tid),
                task_cx_ptr: 0,
                task_status: TaskStatus::Ready,
                exit_code: None,
                time_intr_count: 0,
                total_cpu_cycle_count: 0,
                last_cpu_cycle: 0,
                interrupt_time: 0,
                user_time_us: 0,
                last_user_time_us: 0,
            }),
        }
    }
}

impl PartialEq for TaskControlBlock {
    fn eq(&self, other: &Self) -> bool {
        self.getpid() == other.getpid()
    }
}

impl Eq for TaskControlBlock {}

impl PartialOrd for TaskControlBlock {
    fn partial_cmp(&self, other: &Self) -> Option<core::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for TaskControlBlock {
    fn cmp(&self, other: &Self) -> core::cmp::Ordering {
        self.getpid().cmp(&other.getpid())
    }
}

#[derive(Copy, Clone, PartialEq)]
pub enum TaskStatus {
    Ready,
    Running(usize),
    Zombie,
    Blocking,
}
