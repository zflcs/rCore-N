use spin::Mutex;
use lazy_static::*;
use crate::mm::{FrameTracker, frame_alloc_more, frame_dealloc, PhysPageNum, PageTable, StepByOne, kernel_token, self};
use alloc::vec::Vec;
use virtio_drivers::{Hal, PhysAddr, VirtAddr};

lazy_static! {
    static ref QUEUE_FRAMES: Mutex<Vec<FrameTracker>> =
        unsafe { Mutex::new(Vec::new()) };
}

pub struct VirtioHal;

impl Hal for VirtioHal {
    fn dma_alloc(pages: usize) -> PhysAddr {
        let trakcers = frame_alloc_more(pages);
        let ppn_base = trakcers.as_ref().unwrap().last().unwrap().ppn;
        QUEUE_FRAMES
            .lock()
            .append(&mut trakcers.unwrap());
        let pa: mm::PhysAddr = ppn_base.into();
        pa.0
    }

    fn dma_dealloc(paddr: PhysAddr, pages: usize) -> i32 {
        let pa = PhysAddr::from(paddr);
        let mut ppn_base: PhysPageNum = pa.into();
        for _ in 0..pages {
            frame_dealloc(ppn_base);
            ppn_base.step();
        }
        0
    }

    fn phys_to_virt(paddr: PhysAddr) -> VirtAddr {
        paddr
    }

    fn virt_to_phys(vaddr: VirtAddr) -> PhysAddr {
        PageTable::from_token(kernel_token())
            .translate_va(mm::VirtAddr::from(vaddr))
            .unwrap()
            .0
    }
}
