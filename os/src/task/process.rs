use crate::Result;
use crate::config::USER_STACK_BASE;
use super::INITPROC;
use super::pid::RecycleAllocator;
use crate::fs::{File, Stdin, Stdout};
use crate::mm::{MM, UserBuffer, KERNEL_SPACE, do_mmap, do_munmap, MmapProt, MmapFlags, loader};
use crate::task::pool::insert_into_pid2process;
use crate::task::{add_task, pid_alloc, PidHandle, TaskControlBlock};
use crate::trap::{trap_handler, TrapContext};
use alloc::sync::{Arc, Weak};
use alloc::vec;
use alloc::vec::Vec;
use smoltcp::iface::SocketHandle;
use spin::{Mutex, MutexGuard};

pub struct ProcessControlBlock {
    // immutable
    pub pid: PidHandle,
    // mutable
    inner: Mutex<ProcessControlBlockInner>,
}

pub struct Socket2ktaskinfo(Mutex<Vec<(SocketHandle, (UserBuffer, usize, usize))>>);

impl Socket2ktaskinfo {
    pub fn new() -> Arc<Self> {
        Arc::new(Self(Mutex::new(Vec::new())))
    }

    pub fn lock<'a>(
        self: &'a Arc<Self>,
    ) -> MutexGuard<'a, Vec<(SocketHandle, (UserBuffer, usize, usize))>> {
        self.0.lock()
    }
}

pub struct ProcessControlBlockInner {
    pub is_zombie: bool,
    pub mm: MM,
    pub parent: Option<Weak<ProcessControlBlock>>,
    pub children: Vec<Arc<ProcessControlBlock>>,
    pub exit_code: i32,
    pub fd_table: Vec<Option<Arc<dyn File + Send + Sync>>>,
    pub tasks: Vec<Option<Arc<TaskControlBlock>>>,
    pub task_res_allocator: RecycleAllocator,
    pub socket2ktaskinfo: Arc<Socket2ktaskinfo>,
}

impl ProcessControlBlockInner {
    #[allow(unused)]
    pub fn get_user_token(&self) -> usize {
        self.mm.token()
    }

    pub fn alloc_fd(&mut self) -> usize {
        if let Some(fd) = (0..self.fd_table.len()).find(|fd| self.fd_table[*fd].is_none()) {
            fd
        } else {
            self.fd_table.push(None);
            self.fd_table.len() - 1
        }
    }

    pub fn is_zombie(&self) -> bool {
        self.is_zombie
    }

    pub fn alloc_tid(&mut self) -> usize {
        self.task_res_allocator.alloc()
    }

    pub fn dealloc_tid(&mut self, tid: usize) {
        self.task_res_allocator.dealloc(tid)
    }

    pub fn thread_count(&self) -> usize {
        self.tasks.len()
    }

    pub fn get_task(&self, tid: usize) -> Arc<TaskControlBlock> {
        self.tasks[tid].as_ref().unwrap().clone()
    }

    pub fn mmap(&mut self, start: usize, len: usize, prot: MmapProt, flags: MmapFlags, fd: usize, off: usize) -> Result<usize> {
        do_mmap(&mut self.mm, start.into(), len, prot, flags, fd, off)
    }

    pub fn munmap(&mut self, start: usize, len: usize) -> Result<()> {
        do_munmap(&mut self.mm, start.into(), len)
    }

    pub fn get_socket2ktaskinfo(&self) -> Arc<Socket2ktaskinfo> {
        self.socket2ktaskinfo.clone()
    }
}

impl ProcessControlBlock {
    pub fn acquire_inner_lock(&self) -> MutexGuard<ProcessControlBlockInner> {
        self.inner.lock()
    }

    pub fn empty() -> Arc<Self> {
        Arc::new(Self {
            pid: pid_alloc(),
            inner: Mutex::new(ProcessControlBlockInner { 
                is_zombie: false, 
                mm: MM::new().unwrap(), 
                parent: None, 
                children: Vec::new(), 
                exit_code: 0, 
                fd_table: vec![
                    // 0 -> stdin
                    Some(Arc::new(Stdin)),
                    // 1 -> stdout
                    Some(Arc::new(Stdout)),
                    // 2 -> stderr
                    Some(Arc::new(Stdout)),
                ], 
                tasks: Vec::new(), 
                task_res_allocator: RecycleAllocator::new(), 
                socket2ktaskinfo: Socket2ktaskinfo::new(), 
            }),
        })
    }

    pub fn new(elf_data: &[u8]) -> Arc<Self> {
        // memory_set with elf program headers/trampoline/trap context/user stack
        let mut mm = MM::new().unwrap();
        loader::from_elf(elf_data, &mut mm);
        let entry: usize = mm.entry.into();
        let ustack_base = USER_STACK_BASE;
        // allocate a pid
        let pid_handle = pid_alloc();
        let process = Arc::new(Self {
            pid: pid_handle,
            inner: Mutex::new(ProcessControlBlockInner {
                is_zombie: false,
                mm,
                parent: Some(Arc::downgrade(&INITPROC)),
                children: Vec::new(),
                exit_code: 0,
                fd_table: vec![
                    // 0 -> stdin
                    Some(Arc::new(Stdin)),
                    // 1 -> stdout
                    Some(Arc::new(Stdout)),
                    // 2 -> stderr
                    Some(Arc::new(Stdout)),
                ],
                tasks: Vec::new(),
                task_res_allocator: RecycleAllocator::new(),
                socket2ktaskinfo: Socket2ktaskinfo::new(),
            }),
        });
        // create a main thread, we should allocate ustack and trap_cx here
        let task = Arc::new(TaskControlBlock::new(
            Arc::clone(&process),
            ustack_base,
            true,
        ));

        // prepare trap_cx of main thread
        let task_inner = task.acquire_inner_lock();
        let trap_cx = task_inner.get_trap_cx();
        let ustack_top = task_inner.res.as_ref().unwrap().ustack_top();
        let kstack_top = task.kstack.get_top();
        drop(task_inner);
        *trap_cx = TrapContext::app_init_context(
            entry,
            ustack_top,
            KERNEL_SPACE.lock().token(),
            kstack_top,
            trap_handler as usize,
        );
        // add main thread to the process
        let mut process_inner = process.acquire_inner_lock();
        process_inner.tasks.push(Some(Arc::clone(&task)));
        // log::debug!("{:?}", process_inner.mm);

        drop(process_inner);
        insert_into_pid2process(process.getpid(), Arc::clone(&process));
        // add main thread to scheduler
        add_task(task);
        process
    }

    /// Only support processes with a single thread.
    pub fn exec(self: &Arc<Self>, elf_data: &[u8]) {
        assert_eq!(self.acquire_inner_lock().thread_count(), 1);
        // memory_set with elf program headers/trampoline/trap context/user stack
        let mut mm = MM::new().unwrap();
        loader::from_elf(elf_data, &mut mm);
        let entry: usize = mm.entry.into();
        // substitute memory_set
        let mut process_inner = self.acquire_inner_lock();
        process_inner.mm = mm;
        drop(process_inner);
        // then we alloc user resource for main thread again
        // since memory_set has been changed
        let task = self.acquire_inner_lock().get_task(0);
        let mut task_inner = task.acquire_inner_lock();
        task_inner.res.as_mut().unwrap().ustack_base = USER_STACK_BASE;
        task_inner.res.as_mut().unwrap().alloc_user_res();
        task_inner.trap_cx_ppn = task_inner.res.as_mut().unwrap().trap_cx_ppn();
        let user_sp = task_inner.res.as_mut().unwrap().ustack_top();
        // initialize trap_cx
        let trap_cx = TrapContext::app_init_context(
            // lib_so::user_entry(),
            entry,
            user_sp,
            KERNEL_SPACE.lock().token(),
            task.kstack.get_top(),
            trap_handler as usize,
        );
        // trap_cx.x[10] = args.len();
        // trap_cx.x[11] = argv_base;
        *task_inner.get_trap_cx() = trap_cx;
    }

    /// Only support processes with a single thread.
    pub fn fork(self: &Arc<Self>) -> Arc<Self> {
        let mut parent = self.acquire_inner_lock();
        assert_eq!(parent.thread_count(), 1);
        // clone parent's memory_set completely including trampoline/ustacks/trap_cxs
        let mm = MM::clone(&mut parent.mm).unwrap();
        // alloc a pid
        let pid = pid_alloc();
        // copy fd table
        let mut new_fd_table: Vec<Option<Arc<dyn File + Send + Sync>>> = Vec::new();
        for fd in parent.fd_table.iter() {
            if let Some(file) = fd {
                new_fd_table.push(Some(file.clone()));
            } else {
                new_fd_table.push(None);
            }
        }

        // create child process pcb
        let child = Arc::new(Self {
            pid,
            inner: Mutex::new(ProcessControlBlockInner {
                is_zombie: false,
                mm,
                parent: Some(Arc::downgrade(self)),
                children: Vec::new(),
                exit_code: 0,
                fd_table: new_fd_table,
                tasks: Vec::new(),
                task_res_allocator: RecycleAllocator::new(),
                socket2ktaskinfo: Socket2ktaskinfo::new(),
            }),
        });
        // add child
        parent.children.push(Arc::clone(&child));
        // create main thread of child process
        let task = Arc::new(TaskControlBlock::new(
            Arc::clone(&child),
            parent
                .get_task(0)
                .acquire_inner_lock()
                .res
                .as_ref()
                .unwrap()
                .ustack_base(),
            // here we do not allocate trap_cx or ustack again
            // but mention that we allocate a new kstack here
            false,
        ));
        drop(parent);
        // attach task to child process
        let mut child_inner = child.acquire_inner_lock();
        child_inner.tasks.push(Some(Arc::clone(&task)));
        drop(child_inner);
        // modify kstack_top in trap_cx of this thread
        let task_inner = task.acquire_inner_lock();
        let trap_cx = task_inner.get_trap_cx();
        // log::debug!("{:#X?}", self.acquire_inner_lock().get_task(0).acquire_inner_lock().get_trap_cx());
        // log::debug!("{:#X?}", trap_cx);
        trap_cx.kernel_sp = task.kstack.get_top();
        drop(task_inner);
        insert_into_pid2process(child.getpid(), Arc::clone(&child));
        child
    }

    pub fn getpid(&self) -> usize {
        self.pid.0
    }
}
