use alloc::{
    collections::{BTreeMap, BTreeSet},
    sync::Arc,
};
use spin::{Lazy, Mutex};

use super::{manager::TaskManager, process::ProcessControlBlock, task::TaskControlBlock};

pub struct TaskPool {
    pub scheduler: TaskManager,
}


pub static TASK_POOL: Lazy<Mutex<TaskPool>> = Lazy::new(||Mutex::new(TaskPool::new()));
pub static PID2PCB: Lazy<Mutex<BTreeMap<usize, Arc<ProcessControlBlock>>>> = 
    Lazy::new(|| Mutex::new(BTreeMap::new()));

impl TaskPool {
    pub fn new() -> Self {
        Self {
            scheduler: TaskManager::new(),
        }
    }

    pub fn add(&mut self, task: Arc<TaskControlBlock>) {
        self.scheduler.add(task);
    }

    #[allow(unused)]
    pub fn remove(&mut self, task: Arc<TaskControlBlock>) {
        self.scheduler.remove(&task);
    }


    pub fn fetch(&mut self) -> Option<Arc<TaskControlBlock>> {
        self.scheduler.fetch()
    }

}

pub fn add_task(task: Arc<TaskControlBlock>) {
    TASK_POOL.lock().add(task);
}


pub fn fetch_task() -> Option<Arc<TaskControlBlock>> {
    TASK_POOL.lock().fetch()
}

pub fn pid2process(pid: usize) -> Option<Arc<ProcessControlBlock>> {
    let map = PID2PCB.lock();
    map.get(&pid).map(Arc::clone)
}

pub fn insert_into_pid2process(pid: usize, process: Arc<ProcessControlBlock>) {
    PID2PCB.lock().insert(pid, process);
}

pub fn remove_from_pid2process(pid: usize) {
    let mut map = PID2PCB.lock();
    if map.remove(&pid).is_none() {
        panic!("cannot find pid {} in pid2task!", pid);
    }
}
