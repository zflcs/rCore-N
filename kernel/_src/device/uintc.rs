
use alloc::{vec::Vec, sync::Arc};
use bit_field::BitField;
use id_alloc::{RecycleAllocator, IDAllocator};
use spin::Mutex;
use crate::{mm::{FrameTracker, frame_alloc}, config::PAGE_SIZE};
use lazy_static::lazy_static;
use uintr::*;

pub fn init() {

}


const DEFAULT_UIST_SIZE: usize = 1;

const UISTE_VEC_MASK: u64 = 0xffff << 16;

const UISTE_INDEX_MASK: u64 = 0xffff << 48;

/// User interrupt sender status.
pub struct UIntrSender {
    /// Maximum number of send status table entry.
    limit: usize,

    /// Sender status table allocator.
    alloc: RecycleAllocator,

    /// Frame allocated for send status table
    frames: Vec<FrameTracker>,
}

impl UIntrSender {
    /// Creates a new sender table.
    pub fn new(pages: usize) -> Self {
        let mut frames = Vec::new();
        // 暂时不做错误处理
        frames.resize_with(pages, || frame_alloc().unwrap());
        Self {
            limit: PAGE_SIZE * pages / core::mem::size_of::<UISTE>(),
            alloc: RecycleAllocator::new(0),
            frames,
        }
    }

    /// Gets an entry by index.
    pub fn get(&self, index: usize) -> Option<&'static mut UISTE> {
        if index > self.limit {
            return None;
        }
        let pa = self.frames.first().unwrap().start_address()
            + index * core::mem::size_of::<UISTE>();
        Some(unsafe { &mut *(pa as *mut UISTE) })
    }

    /// Allocates a new [`UISTE`].
    pub fn alloc(&mut self) -> Option<usize> {
        let new = self.alloc.alloc();
        if new < self.limit { Some(new) } else { None }
    }

    /// Deallocates a [`UISTE`].
    pub fn dealloc(&mut self, index: usize) {
        if index < self.limit {
            self.alloc.dealloc(index);
        }
    }
}

/// User interrupt send status table entry.
#[derive(Debug)]
pub struct UISTE(u64);

impl UISTE {
    /// Returns if this entry is valid.
    pub fn is_valid(&self) -> bool {
        (self.0 >> 63) != 0
    }

    /// Enables or disables this entry.
    pub fn set_valid(&mut self, valid: bool) {
        self.0.set_bit(0, valid);
    }

    /// Sets sender vector of this entry.
    pub fn set_vec(&mut self, vec: usize) {
        self.0 &= !UISTE_VEC_MASK;
        self.0 |= ((vec as u64) << 16) & UISTE_VEC_MASK;
    }

    /// Gets sender vector of this entry.
    pub fn get_vec(&self) -> usize {
        ((self.0 & UISTE_VEC_MASK) >> 16) as usize
    }

    /// Sets receiver index of this entry.
    pub fn set_index(&mut self, index: usize) {
        self.0 &= !UISTE_INDEX_MASK;
        self.0 |= ((index as u64) << 48) & UISTE_INDEX_MASK;
    }

    /// Gets receiver index of this entry.
    pub fn get_index(&self) -> usize {
        ((self.0 & UISTE_INDEX_MASK) >> 48) as usize
    }
}

/// Global allocator
lazy_static! {
    pub static ref UINTR_RECEIVER_ALLOC: Arc<Mutex<RecycleAllocator>> = Arc::new(Mutex::new(RecycleAllocator::new(0)));
}

/// User interrupt receiver tracker.
pub struct UIntrReceiverTracker(pub usize);

impl UIntrReceiverTracker {
    pub fn new() -> Self {
        let new = UINTR_RECEIVER_ALLOC.lock().alloc();
        assert!(new < 512);
        Self(new)
    }
}

impl Drop for UIntrReceiverTracker {
    fn drop(&mut self) {
        UINTR_RECEIVER_ALLOC.lock().dealloc(self.0);
    }
}

/// User interrupt receiver status in UINTC
#[repr(C)]
#[derive(Debug)]
pub struct UIntrReceiver {
    /// Kernel defined architecture mode and valid bit.
    mode: u16,

    /// The integer ID of the hardware thread running the code.
    hartid: u16,

    /// Reserved bits.
    _reserved: u32,

    /// One bit for each user interrupt vector. There is user-interrupt request for a vector if the corresponding bit is 1.
    irq: u64,
}

impl UIntrReceiver {
    /// Gets a [`UIntrReceiver`] from UINTC by index.
    pub fn from(index: usize) -> Self {
        assert!(index < UINTC_ENTRY_NUM);
        let low = uintc_read_low(index);
        let high = uintc_read_high(index);
        Self {
            mode: low as u16,
            hartid: (low >> 16) as u16,
            _reserved: 0,
            irq: high,
        }
    }

    /// Synchronize UINTC with this [`UIntrReceiver`].
    pub fn sync(&self, index: usize) {
        let low = (self.mode as u64) | ((self.hartid as u64) << 16);
        let high = self.irq;
        uintc_write_low(index, low);
        uintc_write_high(index, high);
    }
}

/// Task inner member for user interrupt status.
pub struct TaskUIntrInner {
    /// Sender status
    pub uist: Option<UIntrSender>,

    /// Receiver status
    pub uirs: Option<UIntrReceiverTracker>,

    /// Sender vector mask
    pub mask: u64,

    /// User interrupt entry
    pub utvec: usize,

    /// User interrupt handler
    pub uscratch: usize,

    /// User error pc
    pub uepc: usize,
}

impl TaskUIntrInner {
    pub fn new() -> Self {
        Self {
            uist: None,
            uirs: None,
            mask: 0,
            utvec: 0,
            uscratch: 0,
            uepc: 0,
        }
    }

    /// Allocates a sender vector.
    pub fn alloc(&mut self) -> usize {
        let i = self.mask.leading_ones() as usize;
        self.mask.set_bit(i, true);
        i
    }

    /// Deallocates a sender vector
    pub fn dealloc(&mut self, i: usize) {
        self.mask.set_bit(i, false);
    }
}

/// UINTC base
pub const UINTC_BASE: usize = 0x300_0000;

/// UINTC size
pub const UINTC_SIZE: usize = 0x400;

/// Maximum number of UINTC entries
pub const UINTC_ENTRY_NUM: usize = 512;

/// UINTC register width
pub const UINTC_WIDTH: usize = 32;

/* UINTC operations */
pub const UINTC_SEND_OFF: usize = 0x00;
pub const UINTC_LOW_OFF: usize = 0x08;
pub const UINTC_HIGH_OFF: usize = 0x10;
pub const UINTC_ACT_OFF: usize = 0x18;

#[inline(never)]
pub fn uintc_send_uipi(index: usize) {
    assert!(index < UINTC_ENTRY_NUM);
    let pa = UINTC_BASE + index * UINTC_WIDTH + UINTC_SEND_OFF;
    unsafe { *(pa as *mut u64) = 1 };
}
#[inline(never)]
pub fn uintc_read_low(index: usize) -> u64 {
    assert!(index < UINTC_ENTRY_NUM);
    let pa = UINTC_BASE + index * UINTC_WIDTH + UINTC_LOW_OFF;
    unsafe { *(pa as *const u64) }
}
#[inline(never)]
pub fn uintc_write_low(index: usize, data: u64) {
    assert!(index < UINTC_ENTRY_NUM);
    let pa = UINTC_BASE + index * UINTC_WIDTH + UINTC_LOW_OFF;
    unsafe { *(pa as *mut u64) = data };
}
#[inline(never)]
pub fn uintc_read_high(index: usize) -> u64 {
    assert!(index < UINTC_ENTRY_NUM);
    let pa = UINTC_BASE + index * UINTC_WIDTH + UINTC_HIGH_OFF;
    unsafe { *(pa as *const u64) }
}
#[inline(never)]
pub fn uintc_write_high(index: usize, data: u64) {
    assert!(index < UINTC_ENTRY_NUM);
    let pa = UINTC_BASE + index * UINTC_WIDTH + UINTC_HIGH_OFF;
    unsafe { *(pa as *mut u64) = data };
}
#[inline(never)]
pub fn uintc_get_active(index: usize) -> bool {
    assert!(index < UINTC_ENTRY_NUM);
    let pa = UINTC_BASE + index * UINTC_WIDTH + UINTC_ACT_OFF;
    unsafe { *(pa as *const u64) == 0x1 }
}
#[inline(never)]
pub fn uintc_set_active(index: usize) {
    assert!(index < UINTC_ENTRY_NUM);
    let pa = UINTC_BASE + index * UINTC_WIDTH + UINTC_ACT_OFF;
    unsafe { *(pa as *mut u64) = 0x1 };
}


pub const UINTR_TESTCASES: &[&str] = &[
    // "argv",
    "uipi_sample",
    // "pthread_cancel_points",
    // "pthread_cancel",
];
