use alloc::boxed::Box;
use core::future::Future;
use core::pin::Pin;
use core::sync::atomic::{AtomicUsize, Ordering};
use alloc::{sync::Arc, task::Wake};
use core::task::{Waker, Poll};
use spin::Mutex;

/// 协程 Id
#[derive(Eq, PartialEq, Debug, Clone, Copy, Hash, Ord, PartialOrd)]
#[repr(C)]
pub struct CoroutineId(pub usize);

impl CoroutineId {
    pub const EMPTY: Self = Self(usize::MAX);
    /// 生成新的协程 Id
    pub fn generate() -> CoroutineId {
        // 任务编号计数器，任务编号自增
        static COUNTER: AtomicUsize = AtomicUsize::new(0);
        let id = COUNTER.fetch_add(1, Ordering::Relaxed);
        if id > usize::MAX / 2 {
            // TODO: 不让系统 Panic
            panic!("too many tasks!")
        }
        CoroutineId(id)
    }
    /// 根据 usize 生成协程 Id
    pub fn from_val(v: usize) -> Self {
        Self(v)
    }
    /// 获取协程 Id 的 usize
    pub fn get_val(&self) -> usize {
        self.0
    } 
}

/// 协程 waker，在这里只提供一个上下文
pub struct CoroutineWaker(pub CoroutineId);
impl Wake for CoroutineWaker {
    fn wake(self: Arc<Self>) { }
    fn wake_by_ref(self: &Arc<Self>) { }
}
impl CoroutineWaker {
    pub extern "C" fn new(cid: CoroutineId) -> *const Waker {
        let waker = Waker::from(unsafe { Arc::from_raw(&Self(cid)) });
        &waker
    }
}

/// Pin<Box<ffi>>
#[repr(C)]
pub struct FutureFFI{
    pub future: Pin<Box<dyn Future<Output=()> + 'static + Send + Sync>>,
}



/// 协程类型
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
#[repr(C)]
pub enum CoroutineKind {
    /// 内核调度协程
    KernSche,
    /// 内核系统调用协程
    KernSyscall,
    /// 用户协程
    UserNorm,
    ///
    Empty
}

/// 协程，包装了 future，优先级，以及提供上下文的 waker，内核来唤醒或者内核、外部设备发中断，在中断处理程序里面唤醒
#[repr(C)]
pub struct Coroutine {
    /// 协程编号
    pub cid: CoroutineId,
    /// 协程类型
    pub kind: CoroutineKind,
    /// future
    pub inner: Arc<Mutex<CoroutineInner>>,
}

impl Coroutine {
    /// 生成协程
    pub extern "C" fn new(future_ptr: *mut FutureFFI, prio: usize, kind: CoroutineKind) -> *const Self {
        let cid = CoroutineId::generate();
        let task = Self {
            cid,
            kind,
            inner: unsafe { Arc::from_raw(CoroutineInner::new(future_ptr, prio, cid)) },
        };
        &task
    }
}

pub struct CoroutineInner {
    pub future: Box<FutureFFI>,
    pub prio: usize,
}

impl CoroutineInner {
    pub extern "C" fn new(future_ptr: *mut FutureFFI, prio: usize, cid: CoroutineId) -> *const Mutex<Self> {
        let inner = Mutex::new(Self {
            future: unsafe { Box::from_raw(future_ptr) }, 
            prio, 
        });
        &inner
    }
}
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
#[repr(C)]
pub struct CoroutineRes {
    pub cid: CoroutineId,
    pub kind: CoroutineKind,
    pub res: PollRes
}

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
#[repr(C)]
pub enum PollRes {
    Pending,
    Readying,
    Empty
}

impl CoroutineRes {
    pub extern "C" fn new(cid: CoroutineId, kind: CoroutineKind, res: PollRes) -> Self {
        Self { cid, kind, res }
    }
    pub const EMPTY: Self = Self {
        cid: CoroutineId::EMPTY,
        kind: CoroutineKind::Empty,
        res: PollRes::Empty
    };
}