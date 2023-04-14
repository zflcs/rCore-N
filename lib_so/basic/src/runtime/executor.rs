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
use config::{MAX_THREAD_NUM, PRIO_NUM};
use crate::TaskQueue;

/// 进程 Executor
pub struct Executor {
    /// 当前正在运行的协程 Id
    pub currents: [Option<CoroutineId>; MAX_THREAD_NUM],
    /// 协程 map
    pub tasks: Mutex<BTreeMap<CoroutineId, Arc<Coroutine>>>,
    /// 就绪协程队列
    pub ready_queue: [TaskQueue; PRIO_NUM],
    /// 协程优先级位图
    pub bitmap: Mutex<BitMap>,
    /// 执行器线程id
    pub waits: Vec<usize>,
}

impl Executor {
    /// 
    pub const fn new() -> Self {
        Self {
            currents: [None; MAX_THREAD_NUM],
            tasks: Mutex::new(BTreeMap::new()),
            ready_queue: [TaskQueue::EMPTY; PRIO_NUM],
            bitmap: Mutex::new(BitMap::EMPTY),
            waits: Vec::new(),
        }
    }
}

impl Executor {
    /// 暂不提供优先级更新机制
    pub fn reprio(&mut self, _cid: CoroutineId, _prio: usize) {

    }
    /// 添加协程，成功返回优先级是否变化，失败返回插入失败的 cid
    pub fn spawn(&mut self, future: Pin<Box<dyn Future<Output=()> + 'static + Send + Sync>>, prio: usize, kind: CoroutineKind) -> Result<(CoroutineId, bool), CoroutineId> {
        let task = Coroutine::new(future, prio, kind);
        let cid = task.cid;
        let flag = self.ready_queue[prio].is_empty();
        if let Err(cid) = self.ready_queue[prio].enqueue(cid) { 
            Err(cid)
        } else {
            if flag {
                self.bitmap.lock().update(prio, true);
            }
            self.tasks.lock().insert(cid, task);
            Ok((cid, flag))
        }
    }
    
    /// 判断是否还有协程
    pub fn is_empty(&self) -> bool {
        self.tasks.lock().is_empty()
    }
    /// 取出优先级最高的协程 id，成功返回Some((task, need_update))，失败返回 None
    pub fn fetch(&mut self, tid: usize) -> Option<(Arc<Coroutine>, bool)> {
        assert!(tid < MAX_THREAD_NUM);
        for i in 0..PRIO_NUM {
            if let Some(cid) = self.ready_queue[i].dequeue() {
                let task = (*self.tasks.lock().get(&cid).unwrap()).clone();
                self.currents[tid] = Some(cid);
                if self.ready_queue[i].is_empty() {
                    self.bitmap.lock().update(i, false);
                    return Some((task, true));
                } else {
                    return Some((task, false));
                }
            }
        }
        return None;
    }

    /// 增加执行器线程
    pub fn add_wait_tid(&mut self, tid: usize) {
        self.waits.push(tid);
    }

    /// 阻塞协程重新入队，同理，成功返回优先级是否变化，失败返回协程 id
    pub fn re_back(&mut self, cid: CoroutineId, prio: usize) -> Result<(CoroutineId, bool), CoroutineId> {
        let flag = self.ready_queue[prio].is_empty();
        if let Err(cid) = self.ready_queue[prio].enqueue(cid) { 
            Err(cid)
        } else {
            if flag {
                self.bitmap.lock().update(prio, true);
            }
            Ok((cid, flag))
        }
    }
    /// 删除协程
    pub fn del_coroutine(&mut self, cid: CoroutineId) {
        self.tasks.lock().remove(&cid).unwrap();
    }
}