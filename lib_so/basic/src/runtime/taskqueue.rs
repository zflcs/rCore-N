
use core::sync::atomic::AtomicUsize;
use core::sync::atomic::Ordering;
use config::PER_PRIO_COROU;
use heapless::mpmc::MpMcQueue;

use crate::CoroutineId;
type Queue = MpMcQueue<CoroutineId, PER_PRIO_COROU>;
const EMPTY_QUEUE: Queue = Queue::new();
/// 原子队列
pub struct TaskQueue {
    task_count: AtomicUsize,        // 使用 AcqRel 顺序
    queue: Queue
}

impl TaskQueue {
    pub const EMPTY: Self = Self {
        task_count: AtomicUsize::new(0),
        queue: EMPTY_QUEUE
    };
    pub fn is_empty(&self) -> bool {
        self.task_count.load(Ordering::Acquire) == 0
    }
    pub fn enqueue(&mut self, cid: CoroutineId) -> Result<(), CoroutineId>{
        if let Err(item) = self.queue.enqueue(cid) { 
            return Err(item);
        } else {
            self.task_count.fetch_add(1, Ordering::AcqRel);
            return Ok(());
        }
    }
    pub fn dequeue(&mut self) -> Option<CoroutineId> {
        let cid = self.queue.dequeue();
        self.task_count.fetch_sub(1, Ordering::AcqRel);
        cid
    }
}

