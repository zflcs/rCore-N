
#[repr(C)]
#[derive(Debug, Clone)]
pub struct TaskContext {
    pub ra: usize,
    pub s: [usize; 12],
    pub tid: usize,
    pub sp: usize,
}

impl TaskContext {
    pub fn goto_target(target: usize, kstack_top: usize, tid: usize) -> Self {
        Self {
            ra: target as usize,
            s: [0; 12],
            tid,
            sp: kstack_top,
        }
    }
}

impl Default for TaskContext {
    fn default() -> Self {
        Self {
            ra: 0xDEDEDEDE,
            s: [0x23232323; 12],
            tid: 0xDADADADA,
            sp: 0xABABABAB,
        }
    }
}
