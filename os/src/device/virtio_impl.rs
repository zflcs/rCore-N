use core::{
    ptr::NonNull,
    sync::atomic::{AtomicUsize, Ordering},
};
use alloc::{sync::Arc, vec::Vec};
use lazy_static::lazy_static;
use log::trace;
use spin::Mutex;
use virtio_drivers::{BufferDirection, Hal, PAGE_SIZE};
use crate::mm::{FrameTracker, frame_alloc, frame_alloc_more, PhysAddr, PhysPageNum, StepByOne, PageTable,
    kernel_token, VirtAddr, frame_dealloc
};

extern "C" {
    fn end();
}

lazy_static! {
    static ref QUEUE_FRAMES: Arc<Mutex<Vec<FrameTracker>>> = Arc::new(Mutex::new(Vec::new()));
}

pub struct HalImpl;

unsafe impl Hal for HalImpl {

    fn dma_alloc(pages: usize, _direction: BufferDirection) -> (usize, NonNull<u8>) {
        // info!("here");
        let trakcers = frame_alloc_more(pages);
        let ppn_base = trakcers.as_ref().unwrap().last().unwrap().ppn;
        QUEUE_FRAMES
            .lock()
            .append(&mut trakcers.unwrap());
        let pa: PhysAddr = ppn_base.into();
        let paddr = pa.0;
        let vaddr = NonNull::new(paddr as _).unwrap();
        (paddr, vaddr)
    }

    unsafe fn dma_dealloc(paddr: usize, _vaddr: NonNull<u8>, pages: usize) -> i32 {
        let pa = PhysAddr::from(paddr);
        let mut ppn_base: PhysPageNum = pa.into();
        for _ in 0..pages {
            frame_dealloc(ppn_base);
            ppn_base.step();
        }
        0
    }

    unsafe fn mmio_phys_to_virt(paddr: usize, _size: usize) -> NonNull<u8> {
        NonNull::new(paddr as _).unwrap()
    }

    unsafe fn share(buffer: NonNull<[u8]>, _direction: BufferDirection) -> usize {
        let vaddr = buffer.as_ptr() as *mut u8 as usize;
        // Nothing to do, as the host already has access to all memory.
        virt_to_phys(vaddr)
    }

    unsafe fn unshare(_paddr: usize, _buffer: NonNull<[u8]>, _direction: BufferDirection) {
        // Nothing to do, as the host already has access to all memory and we didn't copy the buffer
        // anywhere else.
    }
}

fn virt_to_phys(vaddr: usize) -> usize {
    PageTable::from_token(kernel_token())
        .translate_va(VirtAddr::from(vaddr))
        .unwrap()
        .0
}
