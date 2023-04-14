use super::{TaskControlBlock, get_kernel_prio};
use alloc::collections::{VecDeque, BTreeSet};
use alloc::sync::Arc;
use basic::PRIO_NUM;
use vdso::max_prio;

pub struct TaskManager {
    ready_queue: VecDeque<Arc<TaskControlBlock>>,
    user_intr_process_set: BTreeSet<usize>
}

/// A simple FIFO scheduler.
impl TaskManager {
    pub fn new() -> Self {
        Self {
            ready_queue: VecDeque::new(),
            user_intr_process_set: BTreeSet::new(),
        }
    }
    pub fn add(&mut self, task: Arc<TaskControlBlock>) {
        self.ready_queue.push_back(task);
    }

    pub fn add_user_intr_task(&mut self, pid: usize) {
        self.user_intr_process_set.insert(pid);
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

    pub fn remove_uintr_task(&mut self, pid: usize) {
        self.user_intr_process_set.remove(&pid);
    }

    pub fn fetch(&mut self) -> Option<Arc<TaskControlBlock>> {
        if let Some(kernel_prio) = get_kernel_prio() {
            return None;
        } else {
            let n = self.ready_queue.len();
            if n == 0 { return None; }
            let mut cur;
            let mut cnt = 0;
            if let Some(target_pid) = self.user_intr_process_set.first() {
                loop {
                    cur = self.ready_queue.pop_front().unwrap();
                    let pid = cur.process.upgrade().unwrap().getpid();
                    if pid == *target_pid {
                        return Some(cur);
                    }
                    self.ready_queue.push_back(cur);
                    cnt += 1;
                    if cnt >= n { break; }
                }
                return self.ready_queue.pop_front();
            } else {
                cur = self.ready_queue.pop_front().unwrap();
                cnt = 0;
                let max_prio = cur.process.upgrade().unwrap().get_prio();
                if max_prio.is_none() { return Some(cur); }  // 这个进程被创建了，但尚未开始运行
                let mut max_prio = max_prio.unwrap();
                let mut next;
                loop {
                    if self.ready_queue.is_empty() { break; }
                    next = self.ready_queue.pop_front().unwrap();
                    if let Some(prio) = next.process.upgrade().unwrap().get_prio(){
                        if prio < max_prio {
                            self.ready_queue.push_back(cur);
                            max_prio = prio;
                            cur = next;
                        } else {
                            self.ready_queue.push_back(next);
                        }
                        cnt += 1;
                        if cnt > n { break; }
                    } else {
                        self.ready_queue.push_back(cur);
                        return Some(next);  // 这个进程被创建了，但尚未开始运行
                    }
                }
                return Some(cur);
            }
        }
    }

    #[allow(unused)]
    pub fn prioritize(&mut self, pid: usize) {
        let q = &mut self.ready_queue;
        if q.is_empty() || q.len() == 1 {
            return;
        }
        let front_pid = q.front().unwrap().process.upgrade().unwrap().pid.0;
        if front_pid == pid {
            debug!("[Taskmgr] Task {} already at front", pid);

            return;
        }
        q.rotate_left(1);
        while {
            let f_pid = q.front().unwrap().process.upgrade().unwrap().pid.0;
            f_pid != pid && f_pid != front_pid
        } {
            q.rotate_left(1);
        }
        if q.front().unwrap().process.upgrade().unwrap().pid.0 == pid {
            debug!("[Taskmgr] Prioritized task {}", pid);
        }
    }
}
