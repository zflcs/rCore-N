use super::TaskControlBlock;
use alloc::collections::{BTreeSet, VecDeque};
use alloc::sync::Arc;

pub struct TaskManager {
    ready_queue: VecDeque<Arc<TaskControlBlock>>,
}

/// A simple FIFO scheduler.
impl TaskManager {
    pub fn new() -> Self {
        Self {
            ready_queue: VecDeque::new(),
        }
    }
    pub fn add(&mut self, task: Arc<TaskControlBlock>) {
        self.ready_queue.push_back(task);
    }

    #[allow(unused)]
    pub fn remove(&mut self, task: &Arc<TaskControlBlock>) {
        for (idx, task_item) in self.ready_queue.iter().enumerate() {
            if *task_item == *task {
                self.ready_queue.remove(idx);
                break;
            }
        }
    }

    pub fn fetch(&mut self) -> Option<Arc<TaskControlBlock>> {
        self.ready_queue.pop_front()
    }
    
}
