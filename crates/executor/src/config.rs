/// The amount of thread
pub const MAX_THREAD: usize = 30;

/// The amount of priority
pub const MAX_PRIO: usize = 8;

/// Priority pointer
pub(crate) const PRIO_POINTER: usize = 0x9000_1020;

/// The base addr of cid queue
pub const MESSAGE_QUEUE_ADDR: usize = 0x9000_1040;

/// The length of cid queue
pub const MESSAGE_QUEUE_LEN: usize = 128;