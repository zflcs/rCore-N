use alloc::{
    sync::Arc,
    vec::Vec, collections::VecDeque,
};
use bit_field::BitField;
use core::cell::SyncUnsafeCell;
use kernel_sync::{CPUs, SpinLock};
use spin::Lazy;

use crate::{
    arch::{get_cpu_id, TaskContext, __switch},
    config::*, error::KernelResult,
    // loader::from_args,
};

use super::{Task, TaskState};

pub enum KernTask {
    Corou(Arc<Coroutine>),
    Proc(Arc<Task>)
}

/// Possible interfaces for task schedulers.
pub trait Scheduler {
    /// Add a task to be scheduled sooner or later.
    fn add(&mut self, task: KernTask) -> KernelResult;

    /// Get a task to run on the target processor.
    fn fetch(&mut self) -> Option<KernTask>;
}

// pub struct QueueScheduler {
//     queue: VecDeque<Arc<Task>>,
// }

// impl QueueScheduler {
//     pub fn new() -> Self {
//         Self {
//             queue: VecDeque::new(),
//         }
//     }

//     /// Returns a front-to-back iterator that returns immutable references.
//     pub fn iter(&self) -> vec_deque::Iter<Arc<Task>> {
//         self.queue.iter()
//     }
// }

// impl Scheduler for QueueScheduler {
//     fn add(&mut self, task: Arc<Task>) -> Result<(), error::KernelError> {
//         self.queue.push_back(task);
//         Ok(())
//     }

//     fn fetch(&mut self) -> Option<Arc<Task>> {
//         if self.queue.is_empty() {
//             return None;
//         }

//         let task = self.queue.pop_front().unwrap();

//         // State cannot be set to other states except [`TaskState::Runnable`] by other harts,
//         // e.g. this task is waken up by another task that releases the resources.
//         if task.locked_inner().state != TaskState::RUNNABLE {
//             self.queue.push_back(task);
//             None
//         } else {
//             Some(task)
//         }
//     }
// }

use core::sync::atomic::AtomicUsize;
use errno::Errno;
use executor::{Executor, MAX_PRIO, Coroutine};
use mm_rv::VirtAddr;
use crate::{read_user, config::PRIO_POINTER, error::KernelError};

#[repr(C, align(0x1000))]
pub struct GlobalBitmap(usize);

impl GlobalBitmap {
    fn update(&mut self, bit: usize, value: bool) {
        self.0.set_bit(bit, value);
    }
}

// unsafe impl Sync for GlobalBitmap { }
// unsafe impl Send for GlobalBitmap { }

#[no_mangle]
#[link_section = ".data.executor"]
pub static mut EXECUTOR: Executor = Executor::new();

#[link_section = ".shared.bitmap"]
pub static mut GLOBAL_BITMAP: GlobalBitmap = GlobalBitmap(0);

const EMPTY_QUEUE:  VecDeque<Arc<Task>> = VecDeque::new();
#[repr(C)]
pub struct SharedScheduler {
    bitmap: &'static mut GlobalBitmap,
    executor: &'static mut Executor,
    queue: [VecDeque<Arc<Task>>; MAX_PRIO],
}

impl SharedScheduler {
    pub fn new() -> Self {
        Self {
            bitmap: unsafe { &mut GLOBAL_BITMAP },
            executor: unsafe { &mut EXECUTOR },
            queue: [EMPTY_QUEUE; MAX_PRIO],
        }
    }

    pub fn iter(&self) -> VecDeque<&Arc<Task>> {
        let mut all_task: VecDeque<&Arc<Task>> = VecDeque::new();
        for i in 0..self.queue.len() {
            for t in self.queue[i].iter() {
                all_task.push_back(t);
            }
        }        
        all_task
    }

    pub fn pending(&mut self, c: Arc<Coroutine>) {
        self.executor.pending(c);
    }
}

impl Scheduler for SharedScheduler {
    fn add(&mut self, task: KernTask) -> KernelResult {
        match task {
            KernTask::Proc(t) => {
                let mut mm = t.mm();
                let mut atomic_prio = AtomicUsize::new(0);
                read_user!(mm, VirtAddr::from(PRIO_POINTER), atomic_prio, AtomicUsize).map_err(|e| KernelError::Errno(e))?;
                let prio = atomic_prio.load(core::sync::atomic::Ordering::Relaxed);
                drop(mm);
                self.queue[prio].push_back(t);
                self.bitmap.update(prio, true);
                Ok(())
            },
            KernTask::Corou(c) => {
                let prio = c.inner.lock().prio;
                self.executor.add(c);
                self.bitmap.update(prio, true);
                Ok(())
            }
        }
    }

    fn fetch(&mut self) -> Option<KernTask> {
        for i in 0..MAX_PRIO {
            // there is only ready coroutine in this queue, so only pop once
            if let Some(c) = self.executor.ready_queue[i].pop() {
                return Some(KernTask::Corou(c));
            }
            for _ in 0..self.queue[i].len() {
                if let Some(task) = self.queue[i].pop_front() {
                    // State cannot be set to other states except [`TaskState::Runnable`] by other harts,
                    // e.g. this task is waken up by another task that releases the resources.
                    if task.locked_inner().state != TaskState::RUNNABLE {
                        self.queue[i].push_back(task);
                    } else {
                        return Some(KernTask::Proc(task));
                    }
                }
            }
            // when both two queue has no ready task, we need to update global bitmap
            self.bitmap.update(i, false);
        }
        None
    }
}

/// Reserved for future SMP usage.
pub struct CPUContext {
    /// Current task.
    pub curr: Option<Arc<Task>>,

    /// Idle task context.
    pub idle_ctx: TaskContext,
}

impl CPUContext {
    /// A hart joins to run tasks
    pub fn new() -> Self {
        Self {
            curr: None,
            idle_ctx: TaskContext::zero(),
        }
    }
}

/// Global task manager shared by CPUs.
pub static TASK_MANAGER: Lazy<SpinLock<SharedScheduler>> =
    Lazy::new(|| SpinLock::new(SharedScheduler::new()));

/// Global cpu local states.
pub static CPU_LIST: Lazy<SyncUnsafeCell<Vec<CPUContext>>> = Lazy::new(|| {
    let mut cpu_list = Vec::new();
    for cpu_id in 0..CPU_NUM {
        cpu_list.push(CPUContext::new());
    }
    SyncUnsafeCell::new(cpu_list)
});

/// Returns this cpu context.
pub fn cpu() -> &'static mut CPUContext {
    unsafe { &mut (*CPU_LIST.get())[get_cpu_id()] }
}

/// Gets current task context.
///
/// # Safety
///
/// [`TaskContext`] cannot be modified by other tasks, thus we can access it with raw pointer.
pub unsafe fn curr_ctx() -> *const TaskContext {
    &cpu().curr.as_ref().unwrap().inner().ctx
}

/// IDLE task context on this CPU.
pub fn idle_ctx() -> *const TaskContext {
    &cpu().idle_ctx as _
}

/// Kernel init task which will never be dropped.
pub static INIT_TASK: Lazy<Arc<Task>> = Lazy::new(|| Arc::new(Task::init().unwrap()));

/// Reclaim resources delegated to [`INIT_TASK`].
pub fn init_reclaim() {
    let mut init = INIT_TASK.locked_inner();
    init.children.clear();
}

/// IDLE task:
///
/// 1. Each cpu tries to acquire the lock of global task manager.
/// 2. Each cpu runs the task fetched from schedule queue.
/// 3. Handle the final state after a task finishes `do_yield` or `do_exit`.
/// 4. Reclaim resources handled by [`INIT_TASK`].
pub unsafe fn idle() -> ! {
    loop {
        init_reclaim();

        let mut task_manager = TASK_MANAGER.lock();

        if let Some(task) = task_manager.fetch() {
            match task {
                KernTask::Proc(t) => {
                    let next_ctx = {
                        let mut locked_inner = t.locked_inner();
                        locked_inner.state = TaskState::RUNNING;
                        &t.inner().ctx as *const TaskContext
                    };
                    log::trace!("Run {:?}", t);
                    // Ownership moved to `current`.
                    cpu().curr = Some(t);
        
                    // Release the lock.
                    drop(task_manager);
                    __switch(idle_ctx(), next_ctx);
                    if cpu().curr.is_some() {
                        let cur = cpu().curr.take().unwrap();
                        match cur.get_state() {
                            TaskState::RUNNABLE => {TASK_MANAGER.lock().add(KernTask::Proc(cur)); },
                            _ => {},
                        };
                    }
                },
                KernTask::Corou(c) => {
                    drop(task_manager);
                    if c.clone().execute().is_pending() {
                        TASK_MANAGER.lock().pending(c);
                    }
                }
            }
        }
    }
}

/// Current task suspends. Run next task.
///
/// # Safety
///
/// Unsafe context switch will be called in this function.
pub unsafe fn do_yield() {
    let curr = cpu().curr.as_ref().unwrap();
    // log::trace!("{:#?} suspended", curr);
    let curr_ctx = {
        let mut locked_inner = curr.locked_inner();
        locked_inner.state = TaskState::RUNNABLE;
        &curr.inner().ctx as *const TaskContext
    };

    // Saves and restores CPU local variable, intena.
    let intena = CPUs[get_cpu_id()].intena;
    __switch(curr_ctx, idle_ctx());
    CPUs[get_cpu_id()].intena = intena;
}

/// block current task
pub unsafe fn do_block() {
    let curr = cpu().curr.as_ref().unwrap();
    log::debug!("{:#?} block", curr);
    let curr_ctx = {
        let mut locked_inner = curr.locked_inner();
        locked_inner.state = TaskState::INTERRUPTIBLE;
        &curr.inner().ctx as *const TaskContext
    };

    // Saves and restores CPU local variable, intena.
    let intena = CPUs[get_cpu_id()].intena;
    __switch(curr_ctx, idle_ctx());
    CPUs[get_cpu_id()].intena = intena;
}
