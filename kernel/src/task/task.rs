use alloc::{
    collections::LinkedList,
    string::String,
    sync::{Arc, Weak},
    vec::Vec,
};
use core::{cell::SyncUnsafeCell, fmt};
use errno::Errno;
use id_alloc::*;
use kernel_sync::{SpinLock, SpinLockGuard};
use log::{trace, info};
use signal_defs::*;
use spin::Lazy;
use syscall_interface::AT_FDCWD;
use vfs::Path;

use crate::{
    arch::{
        TaskContext, __switch,
        mm::*,
        trap::{user_trap_handler, user_trap_return, TrapFrame},
    },
    config::*,
    error::{KernelError, KernelResult},
    fs::{FDManager, FSInfo},
    loader::from_elf,
    mm::{KERNEL_MM, MM},
    task::sched::Scheduler,
};

use crate::arch::uintr::*;

use super::*;

bitflags::bitflags! {
    /// Five-state model:
    ///
    /// - **Running** or **Runnable** (R)
    /// - **Sleeping** states: **Interruptible** (S) and **Uninterruptible** (D).
    /// - **Stopped** (T)
    /// - **Zombie** (Z)
    pub struct TaskState: u8 {
        /// The task is waiting in scheduler.
        const RUNNABLE = 1 << 0;

        /// The task takes up a CPU core to execute its code.
        const RUNNING = 1  << 1;

        /// A task will react to `SIGSTOP` or `SIGTSTP` signals and be brought back
        /// to running or runnable by `SIGCONT` signal.
        const STOPPED = 1 << 2;

        /// Task will only for resources to be available.
        const INTERRUPTIBLE = 1 << 3;

        /// Task will react to both signals and the availability of resources.
        const UNINTERRUPTIBLE = 1 << 4;

        /// When a task has completed its execution or is terminated, it will send the
        /// `SIGCHLD` signal to the parent task and go into the zombie state.
        const ZOMBIE = 1 << 5;

        /// Task dead
        const DEAD = 1 << 6;
    }
}

/// Task identifier tracker
#[derive(Debug, PartialEq, Eq)]
pub struct TID(pub usize);

impl TID {
    /// Creates a new [`TID`].
    pub fn new() -> Self {
        Self(TID_ALLOCATOR.lock().alloc())
    }
}

impl Drop for TID {
    fn drop(&mut self) {
        TID_ALLOCATOR.lock().dealloc(self.0)
    }
}

/// Global allocator for [`TID`].
static TID_ALLOCATOR: Lazy<SpinLock<RecycleAllocator>> =
    Lazy::new(|| SpinLock::new(RecycleAllocator::new(1)));

/// A wrapper for kernel stack.
pub struct KernelStack(AllocatedFrameRange);

impl KernelStack {
    /// Creates a new kernel stack.
    pub fn new() -> KernelResult<Self> {
        Ok(Self(
            AllocatedFrameRange::new(KERNEL_STACK_PAGES, true)
                .map_err(|_| KernelError::FrameAllocFailed)?,
        ))
    }

    /// Returns base address of [`KernelStack`].
    pub fn base(&self) -> usize {
        self.0.start_address().value() + KERNEL_STACK_SIZE - ADDR_ALIGN
    }

    /// Returns top address of [`KernelStack`].
    pub fn top(&self) -> usize {
        self.0.start_address().value()
    }
}

/// Trap frame tracker
pub struct TrapFrameTracker(pub PhysAddr);

impl Drop for TrapFrameTracker {
    fn drop(&mut self) {
        frame_dealloc(Frame::from(self.0).number(), 1);
    }
}

/// Mutable inner data of the task, not protected by lock.
pub struct TaskInner {
    /// Task exit code, known as the number returned to a parent process by an executable.
    pub exit_code: i32,

    /// Task context
    pub ctx: TaskContext,

    /// Kernel stack
    pub kstack: KernelStack,

    /// If a thread is started using `clone(2)` with the `CLONE_CHILD_SETTID` flag,
    /// set_child_tid is set to the value passed in the ctid argument of that system call.
    ///
    /// When set_child_tid is set, the very first thing the new thread does is to write
    /// its thread ID at this address.
    pub set_child_tid: usize,

    /// If a thread is started using `clone(2)` with the `CLONE_CHILD_CLEARTID` flag,
    /// clear_child_tid is set to the value passed in the ctid argument of that system call.
    pub clear_child_tid: usize,

    /// Pending signals.
    pub sig_pending: SigPending,

    /// Blocked signals.
    pub sig_blocked: SigSet,

    /* Shared and mutable */
    /// Address space metadata.
    pub mm: Arc<SpinLock<MM>>,

    /// File descriptor table.
    pub files: Arc<SpinLock<FDManager>>,
}

unsafe impl Send for TaskInner {}

/// Mutable inner data of the task, protected by lock.
pub struct TaskLockedInner {
    /// Task state, using five-state model.
    pub state: TaskState,

    /// Sleep lock id.
    pub sleeping_on: Option<usize>,

    /// Hierarchy pointers in task management.
    /// INIT task has no parent task.
    pub parent: Option<Weak<Task>>,

    /// Pointers to child tasks.
    /// When a parent task exits before its children, they will become orphans.
    /// These tasks will be adopted by INIT task to avoid being dropped when the reference
    /// counter becomes 0.
    pub children: LinkedList<Arc<Task>>,
    // /// Linkage in my parent's children list
    // pub sibling: Option<CursorMut<'static, Arc<Task>>>,
}

unsafe impl Send for TaskLockedInner {}

/// In conventional opinion, process is the minimum unit of resource allocation, while task (or
/// thread) is the minimum unit of scheduling. Process is always created with a main task. On
/// the one hand, a process may have several tasks; on the other hand, these tasks shared the
/// same information belonging to the process, such as virtual memory handler, process
/// identification (called pid) and etc.
///
/// We use four types of regions to maintain the task metadata:
/// - Shared with other takss and mutable: uses [`Arc<SpinLock<T>>`]
/// - Local and immutable: data initialized once when task created
/// - Local and mutable fields that might be changed by other harts: uses [`SpinLock<TaskLockedInner>`] to wrap
/// the data together
/// - Local and mutable files that cannot be accessed by multiple harts at the same time: uses
/// [`SyncUnsafeCell<TaskInner>`]
///
/// # Thread Group
///
/// The threads within a group can be distinguished by their (system-wide) unique thread IDs (TID).
/// A new thread's TID is available as the function result returned to the caller, and a thread can
/// obtain its own TID using gettid(2).
///
/// If any of the threads in a thread group performs an execve(2), then all threads other than the thread
/// group leader are terminated, and the new program is executed in the thread group leader.
///
/// If one of the threads in a thread group creates a child using fork(2), then any thread in the group
/// can wait(2) for that child.
///
/// Signal dispositions and actions are process-wide: if an unhandled signal is delivered to a thread,
/// then it will affect (terminate, stop, continue, be ignored in) all members of the thread group.
/// Each thread has its own signal mask, as set by sigprocmask(2).
pub struct Task {
    /* Local and immutable */
    /// Name of this task (for debug).
    pub name: String,

    /// Task identifier (system-wide unique)
    pub tid: TID,

    /// Process identifier (same as the group leader)
    pub pid: usize,

    /// Trapframe physical address.
    pub trapframe: Option<TrapFrameTracker>,

    /// Signal (usually SIGCHLD) sent when task exits.
    pub exit_signal: usize,

    /* Shared and mutable */
    /// Filesystem info
    pub fs_info: Arc<SpinLock<FSInfo>>,

    /// Signal actions.
    pub sig_actions: Arc<SpinLock<SigActions>>,

    /* Local and mutable */
    /// Inner data wrapped by [`SpinLock`].
    pub locked_inner: SpinLock<TaskLockedInner>,

    /// Inner data wrapped by [`SyncUnsafeCell`].
    pub inner: SyncUnsafeCell<TaskInner>,

    /// Inner data wrapped by [`SyncUnsafeCell`].
    pub uintr_inner: SyncUnsafeCell<TaskUIntrInner>,
}

impl Task {
    /// Returns an empty task without address space. trap frame and task context,
    /// which will not be scheduled.
    pub fn init() -> KernelResult<Self> {
        Ok(Self {
            name: String::from("init"),
            tid: TID(0),
            pid: 0,
            trapframe: None,
            exit_signal: SIGNONE,
            fs_info: Arc::new(SpinLock::new(FSInfo {
                umask: 0,
                cwd: String::from("/"),
                root: String::from("/"),
            })),
            sig_actions: Arc::new(SpinLock::new([SigAction::default(); NSIG])),
            locked_inner: SpinLock::new(TaskLockedInner {
                state: TaskState::RUNNABLE,
                sleeping_on: None,
                parent: None,
                children: LinkedList::new(),
            }),
            inner: SyncUnsafeCell::new(TaskInner {
                exit_code: 0,
                ctx: TaskContext::zero(),
                kstack: KernelStack::new()?,
                set_child_tid: 0,
                clear_child_tid: 0,
                sig_pending: SigPending::new(),
                sig_blocked: SigSet::new(),
                mm: Arc::new(SpinLock::new(MM::new()?)),
                files: Arc::new(SpinLock::new(FDManager::new())),
            }),
            uintr_inner: SyncUnsafeCell::new(TaskUIntrInner::new()),
        })
    }
    /// Create a new task from ELF data.
    pub fn new(dir: String, elf_data: &[u8], args: Vec<String>) -> KernelResult<Self> {
        let name = args.join(" ");

        let mut mm = MM::new()?;
        let sp = from_elf(elf_data, args, &mut mm)?;
        
        trace!("\nTask [{}]\n{:#?}", &name, mm);

        let kstack = KernelStack::new()?;
        let kstack_base = kstack.base();

        let tid = TID::new();
        let tid_num = tid.0;

        let trapframe_pa = init_trapframe(&mut mm, tid.0)?;
        let trapframe = TrapFrame::from(trapframe_pa);
        *trapframe = TrapFrame::new(
            KERNEL_MM.lock().page_table.satp(),
            kstack.base(),
            user_trap_handler as usize,
            mm.entry.value(),
            sp.into(),
        );

        let fd_manager = FDManager::new();
        let task = Self {
            name,
            tid,
            pid: tid_num,
            trapframe: Some(TrapFrameTracker(trapframe_pa)),
            exit_signal: SIGNONE,
            fs_info: Arc::new(SpinLock::new(FSInfo {
                umask: 0,
                cwd: dir,
                root: String::from("/"),
            })),
            sig_actions: Arc::new(SpinLock::new([SigAction::default(); NSIG])),
            inner: SyncUnsafeCell::new(TaskInner {
                exit_code: 0,
                ctx: TaskContext::new(user_trap_return as usize, kstack_base),
                kstack,
                set_child_tid: 0,
                clear_child_tid: 0,
                sig_pending: SigPending::new(),
                sig_blocked: SigSet::new(),
                mm: Arc::new(SpinLock::new(mm)),
                files: Arc::new(SpinLock::new(fd_manager)),
            }),
            locked_inner: SpinLock::new(TaskLockedInner {
                state: TaskState::RUNNABLE,
                sleeping_on: None,
                parent: None,
                children: LinkedList::new(),
            }),
            uintr_inner: SyncUnsafeCell::new(TaskUIntrInner::new()),
        };
        Ok(task)
    }

    /// Returns the [`TrapFrame`] of this task
    pub fn trapframe(&self) -> &'static mut TrapFrame {
        TrapFrame::from(self.trapframe.as_ref().unwrap().0)
    }

    /// Mutable access to [`TaskInner`].
    pub fn inner(&self) -> &mut TaskInner {
        unsafe { &mut *self.inner.get() }
    }

    /// Acquires inner lock to modify the metadata in [`TaskLockedInner`].
    pub fn locked_inner(&self) -> SpinLockGuard<TaskLockedInner> {
        self.locked_inner.lock()
    }

    /// Mutable access to [`TaskUIntrInner`].
    pub fn uintr_inner(&self) -> &mut TaskUIntrInner {
        unsafe { &mut *self.uintr_inner.get() }
    }

    /// Acquires inner lock to modify [`MM`].
    pub fn mm(&self) -> SpinLockGuard<MM> {
        self.inner().mm.lock()
    }

    /// Acquires inner lock to modify [`FDManager`].
    pub fn files(&self) -> SpinLockGuard<FDManager> {
        self.inner().files.lock()
    }

    /// Gets the directory name from a file descriptor.
    pub fn get_dir(&self, dirfd: usize) -> KernelResult<Path> {
        if dirfd == AT_FDCWD {
            Ok(Path::new(self.fs_info.lock().cwd.as_str()))
        } else {
            let dir = self.files().get(dirfd)?;
            if dir.is_dir() {
                Ok(dir.get_path().unwrap())
            } else {
                Err(KernelError::Errno(Errno::ENOTDIR))
            }
        }
    }

    /// Returns the task state if the inner lock is not held, otherwise returns
    /// [`TaskState::RUNNING`] or [`TaskState::INTERRUPTIBLE`].
    /// In our implementations, the inner lock might be held in sleep lock context.
    pub fn get_state(&self) -> TaskState {
        if let Some(inner) = self.locked_inner.try_lock() {
            inner.state
        } else {
            TaskState::RUNNING | TaskState::INTERRUPTIBLE
        }
    }
}

impl Drop for Task {
    fn drop(&mut self) {
        trace!("Drop {:?}", self);
    }
}

impl fmt::Debug for Task {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Task [{}] pid={} tid={}",
            self.name, self.pid, self.tid.0
        )
    }
}

/// Returns trapframe base of the task in the address space by task identification.
///
/// Trapframes are located right below the Trampoline in each address space.
pub fn trapframe_base(tid: usize) -> usize {
    TRAMPOLINE_VA - PAGE_SIZE - tid * PAGE_SIZE
}

/// Initialize trapframe
pub fn init_trapframe(mm: &mut MM, tid: usize) -> KernelResult<PhysAddr> {
    let trapframe = AllocatedFrame::new(true).map_err(|_| KernelError::FrameAllocFailed)?;
    let trapframe_pa = trapframe.start_address();
    let trapframe_va: VirtAddr = trapframe_base(tid).into();
    mm.page_table
        .map(
            Page::from(trapframe_va),
            trapframe.clone(),
            PTEFlags::READABLE | PTEFlags::WRITABLE | PTEFlags::VALID | PTEFlags::ACCESSED | PTEFlags::DIRTY,
        )
        .map_err(|_| KernelError::PageTableInvalid)?;
    // Will be manually dropped
    core::mem::forget(trapframe);
    Ok(trapframe_pa)
}

/// Returns task stack layout [top, base) by task identification.
///
/// Stack grows from high address to low address.
pub fn ustack_layout(tid: usize) -> (usize, usize) {
    let ustack_base = USER_STACK_BASE - tid * (USER_STACK_SIZE + PAGE_SIZE);
    let ustack_top = ustack_base - USER_STACK_SIZE;
    (ustack_top, ustack_base - ADDR_ALIGN)
}

/* Sleep lock */

impl kernel_sync::SleepLockSched for TaskLockedInner {
    unsafe fn sched(guard: SpinLockGuard<Self>) {
        // Lock might be released after the task is pushed back to the scheduler.
        TASK_MANAGER.lock().add(KernTask::Proc(cpu().curr.clone().unwrap()));
        drop(guard);

        __switch(curr_ctx(), idle_ctx());
    }

    fn set_id(task: &mut Self, id: Option<usize>) {
        task.sleeping_on = id;
    }

    fn sleep(task: &mut Self) {
        task.state = TaskState::INTERRUPTIBLE;
    }

    /// Wakes up tasks sleeping on this lock.
    fn wakeup(id: usize) {
        TASK_MANAGER.lock().iter().iter().for_each(|task| {
            let mut inner = task.locked_inner();
            if inner.state == TaskState::INTERRUPTIBLE
                && inner.sleeping_on.is_some()
                && inner.sleeping_on.unwrap() == id
            {
                inner.state = TaskState::RUNNABLE;
            }
        });
    }
}

impl Task {
    /// Signal is ignored for this task.
    pub fn sig_ignored(&self, sig_actions: &SigActions, sig: usize) -> bool {
        /*
         * Blocked signals are never ignored, since the
         * signal handler may change by the time it is
         * unblocked.
         */
        if self.inner().sig_blocked.get(sig - 1) {
            return false;
        }

        sig_actions[sig - 1].handler == SIG_IGN
            || (sig_actions[sig - 1].handler == SIG_DFL && sig_kernel_ignore(sig))
    }
}
