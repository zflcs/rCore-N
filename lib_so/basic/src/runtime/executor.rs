use alloc::collections::BTreeMap;
use alloc::sync::Arc;
use alloc::vec::Vec;
use spin::Mutex;
use super::{
    coroutine::{Coroutine, CoroutineId, CoroutineKind},
    BitMap,
};
use alloc::boxed::Box;
use core::pin::Pin;
use core::future::Future;
use crate::{MAX_THREAD_NUM, PRIO_NUM, PER_PRIO_COROU};

use heapless::mpmc::MpMcQueue;
pub type FreeLockQueue = MpMcQueue<CoroutineId, PER_PRIO_COROU>;
const QUEUE_CONST: FreeLockQueue = FreeLockQueue::new();

/// 进程 Executor
pub struct Executor {
    /// 当前正在运行的协程 Id，不同的线程操作不同的位置，不需要加锁
    pub currents: [Option<CoroutineId>; MAX_THREAD_NUM],            
    /// 协程 map，多线程，需要加锁
    pub tasks: Mutex<BTreeMap<CoroutineId, Arc<Coroutine>>>,       
    /// 就绪协程队列，无锁队列，不需要加锁
    pub ready_queue: [FreeLockQueue; PRIO_NUM],
    /// 协程优先级位图，需要加锁
    pub bitmap: BitMap,
    /// 执行器线程 id，需要加锁
    pub waits: Mutex<Vec<usize>>,
}

impl Executor {
    /// 
    pub const fn new() -> Self {
        Self {
            currents: [None; MAX_THREAD_NUM],       
            tasks: Mutex::new(BTreeMap::new()),                 
            ready_queue: [QUEUE_CONST; PRIO_NUM],
            bitmap: BitMap::new(),
            waits: Mutex::new(Vec::new()),
        }
    }
}

impl Executor {
    /// 更新协程优先级，暂不提供更新优先级接口
    pub fn reprio(&mut self, _cid: CoroutineId, _prio: usize) {
        // let _lock = self.wr_lock.lock();
        // let task = self.tasks.get(&cid).unwrap();
        // // task.inner.lock().prio = prio;
        // let p = task.inner.lock().prio;
        // // 先从队列中出来
        // if let Ok(idx) = self.ready_queue[p].binary_search(&cid){
        //     self.ready_queue[p].remove(idx);
        //     if self.ready_queue[p].is_empty() {
        //         self.bitmap.update(p, false);
        //     }
        // }
        // task.inner.lock().prio = prio;
        // self.ready_queue[prio].push_back(cid);
        // self.bitmap.update(prio, true);
        // self.priority = self.bitmap.get_priority();
    }
    /// 是否存在协程
    pub fn is_empty(&self) -> bool {
        self.tasks.lock().is_empty()
    }
    /// 获取位图
    pub fn get_bitmap(&self) -> usize {
        self.bitmap.get_val()
    }
    /// 添加协程
    pub fn spawn(&mut self, future: Pin<Box<dyn Future<Output=()> + 'static + Send + Sync>>, prio: usize, kind: CoroutineKind) -> usize {
        let task = Coroutine::new(future, prio, kind);
        let cid = task.cid;
        while let Err(_) = self.ready_queue[prio].enqueue(cid) { }
        self.tasks.lock().insert(cid, task);
        self.bitmap.update(prio, true);
        return cid.0;
    }
    
    /// 取出优先级最高的协程 id，并且更新位图
    pub fn fetch(&mut self, tid: usize) -> Option<Arc<Coroutine>> {
        assert!(tid < MAX_THREAD_NUM);
        for i in 0..PRIO_NUM {
            if let Some(cid) = self.ready_queue[i].dequeue() {
                let task = (*self.tasks.lock().get(&cid).unwrap()).clone();
                self.currents[tid] = Some(cid);
                return Some(task);
            } else {
                self.bitmap.update(i, false);
            }
        }
        return None;
    }

    /// 增加执行器线程
    pub fn add_wait_tid(&mut self, tid: usize) {
        self.waits.lock().push(tid);
    }

    /// 阻塞协程重新入队
    pub fn wake(&mut self, cid: CoroutineId) -> usize {
        let prio = self.tasks.lock().get(&cid).unwrap().inner.lock().prio;
        while let Err(_) = self.ready_queue[prio].enqueue(cid) { }
        self.bitmap.update(prio, true);
        prio
    }
    /// 删除协程，协程已经被执行完了，应该在此处更新位图，
    /// 但是由于无锁队列没有 is_empty() 函数，因此不太方便进行处理，这里可能会导致优先级更新不及时
    /// 当被删除的协程的优先级就绪队列中还存在协程时，不会带来影响
    /// 没有协程时，若删除之后正好被切换，此时没有更新优先级，可能会导致线程被多调度一次，会影响到其他的进程、线程的调度
    /// 当低优先级的队列中存在协程时，它会被误认为还存在高优先级，此时调度一次之后，检测到还有更高优先级的进程存在，这时会让出 CPU 的权限，这里多增加一次切换开销
    pub fn del_coroutine(&mut self, cid: CoroutineId) {
        self.tasks.lock().remove(&cid);
        // TODO：更新优先级
    }
}