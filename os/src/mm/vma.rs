
use super::address::SimpleRange;
use super::{frame_alloc, VPNRange, VirtPageNum, PhysPageNum};
use super::{
    flags::VMFlags,
    VirtAddr,
    page_count, page_index,
    FrameTracker,
    PageTable, PTEFlags, PageTableEntry,
};
use alloc::{sync::Arc, vec::Vec};
use crate::config::USER_MAX_PAGES;
use crate::Result;

/// Represents an area in virtual address space with the range of [start_va, end_va).
pub struct VMArea {
    /// Access flags of this area.
    pub flags: VMFlags,

    /// Start virtual address.
    pub start_va: VirtAddr,

    /// End virtual address.
    pub end_va: VirtAddr,

    /// Mapped to a allocated frames.
    pub frames: Vec<Option<Arc<FrameTracker>>>,

    // /// Backed by file wihch can be None.
    // pub file: Option<Arc<MmapFile>>,
}

impl VMArea {
    /// Creates a new virtual memory area [start_va, end_va) with protection flags.
    pub fn new(
        start_va: VirtAddr,
        end_va: VirtAddr,
        flags: VMFlags,
        frames: Vec<Option<Arc<FrameTracker>>>,
        // file: Option<Arc<MmapFile>>,
    ) -> Result<Self> {
        if end_va <= start_va || flags.is_empty() {
            log::error!("invalid vma args");
            return Err(());
        }
        Ok(Self {
            flags,
            start_va,
            end_va,
            frames,
            // file,
        })
    }

    /// Creates a new [`VMArea`] with frames allocated lazily.
    pub fn new_lazy(
        start_va: VirtAddr,
        end_va: VirtAddr,
        flags: VMFlags,
        // file: Option<Arc<MmapFile>>,
    ) -> Result<Self> {
        let count = page_count(start_va, end_va);
        if end_va <= start_va || flags.is_empty() || count == 0 || count > USER_MAX_PAGES {
            log::error!("invalid vma args");
            return Err(());
        }
        let mut frames = Vec::new();
        frames.resize_with(count, || None);

        Ok(Self {
            flags,
            start_va,
            end_va,
            frames,
            // file,
        })
    }

    /// Creates a new [`VMArea`] with frames allocated in advance.
    pub fn new_fixed(start_va: VirtAddr, end_va: VirtAddr, flags: VMFlags) -> Result<Self> {
        let count = page_count(start_va, end_va);
        if end_va <= start_va || flags.is_empty() || count == 0 || count > USER_MAX_PAGES {
            log::error!("invalid vma args");
            return Err(());
        }
        let mut frames = Vec::new();
        if !flags.contains(VMFlags::IDENTICAL) {
            frames.resize_with(count, || frame_alloc().map(Arc::new));
        }

        Ok(Self {
            flags,
            start_va,
            end_va,
            frames,
            // file: None,
        })
    }

    /// Returns the size of this [`VMArea`] in pages.
    pub fn size_in_pages(&self) -> usize {
        page_count(self.start_va, self.end_va)
    }

    /// Returns if this area contains the virtual address.
    pub fn contains(&self, va: VirtAddr) -> bool {
        self.start_va <= va && self.end_va > va
    }

    /// Returns if this area covers the given virtual address range.
    pub fn covers(&self, start_va: VirtAddr, end_va: VirtAddr) -> bool {
        self.start_va <= start_va && self.end_va > end_va && start_va < end_va
    }

    /// Extends an area with new end.
    ///
    /// This function does not check if current area overlaps with an old area, thus  
    /// the result is unpredictable. So it is marked as `unsafe` for further use.
    pub unsafe fn extend(&mut self, new_end: VirtAddr) {
        self.end_va = new_end;
        self.frames.resize_with(self.size_in_pages(), || None);
    }

    /// Gets the frame by index.
    pub fn get_frame(&mut self, index: usize, alloc: bool) -> Option<Arc<FrameTracker>> {
        if let Some(ppn) = &self.frames[index] {
            Some((*ppn).clone())
        } else if alloc {
            let ppn = frame_alloc().map(Arc::new).unwrap();
            // ownership moved
            self.frames[index] = Some(ppn.clone());
            Some(ppn)
        } else {
            None
        }
    }

    /// Reclaims the frame by index, writing back to file if before the [`AllocatedFrame`] dropped.
    pub fn reclaim_frame(&mut self, index: usize) -> Option<Arc<FrameTracker>> {
        if let Some(frame) = self.frames[index].take() {
            Some(frame)
        } else {
            None
        }
    }

    /// Gets all frames of this [`VMArea`].
    pub fn get_frames(&mut self, alloc: bool) -> Result<Vec<Option<Arc<FrameTracker>>>> {
        if self.flags.contains(VMFlags::IDENTICAL) {
            Ok(self.frames.clone())
        } else {
            let mut v = Vec::new();
            for ppn in &mut self.frames {
                if ppn.is_some() {
                    let p = ppn.as_ref().unwrap();
                    v.push(Some((*p).clone()));
                } else {
                    if alloc {
                        let new_ppn = ppn.insert(frame_alloc().map(Arc::new).unwrap());
                        v.push(Some((*new_ppn).clone()))
                    } else {
                        v.push(None);
                    }
                }
            }
            Ok(v)
        }
    }

    /// Maps the whole virtual memory area.
    ///
    /// Notice that this function will allocate frames directly to create map.
    ///
    /// This function flushes TLB entries each page, thus there is no need to
    /// call [`Self::flush_all`] explicitly.
    pub fn map_all(&mut self, pt: &mut PageTable, flags: PTEFlags, alloc: bool) -> Result<()> {
        use core::iter::zip;
        let vpn_range = VPNRange::new(self.start_va.into(), self.end_va.into());
        if self.flags.contains(VMFlags::IDENTICAL) {
            for vpn in vpn_range {
                pt.map(vpn, PhysPageNum(vpn.0), PTEFlags::V | flags);
            }
        } else {
            for (vpn, ppn) in zip(vpn_range, self.get_frames(alloc)?) {
                if ppn.is_some() {
                    // log::debug!("{:?}, {:?}", vpn, ppn.clone().unwrap().clone().ppn);

                    pt.map(vpn, ppn.unwrap().ppn, PTEFlags::V | flags);
                }
            }
        }
        unsafe { riscv::asm::sfence_vma_all(); }
        Ok(())
    }

    /// Unmaps the whole virtual memory area, escaping errors.
    ///
    /// This function flushes TLB entries each page, thus there is no need to
    /// call [`Self::flush_all`] explicitly.
    pub fn unmap_all(&self, pt: &mut PageTable) -> Result<()> {
        let vpn_range = VPNRange::new(self.start_va.into(), self.end_va.into());
        for vpn in vpn_range {
            pt.unmap(vpn);
        }
        unsafe { riscv::asm::sfence_vma_all(); }
        Ok(())
    }

    /// Allocates a frame for mapped page.
    ///
    pub fn alloc_frame(&mut self, vpn: VirtPageNum, pt: &mut PageTable) -> Result<Arc<FrameTracker>> {
        let mut pte = pt.find_pte_create(vpn).unwrap();
        if !pte.is_valid() || (!pte.flags().contains(PTEFlags::W) && self.flags.contains(VMFlags::WRITE)) {
            let index = vpn.0 - VirtPageNum::from(self.start_va).0;
            let frame = if pte.is_valid() {
                let old = self.get_frame(index, false).unwrap();
                // we don't drop the old frame immediately, for it can be allocated again as new frame
                let need_drop = self.reclaim_frame(index);
                let old_content = need_drop.unwrap().ppn.get_bytes_array();
                let new = self.get_frame(index, true).unwrap();
                let new_content = new.ppn.get_bytes_array();
                new_content.copy_from_slice(old_content);
                new
            } else {
                self.get_frame(index, true).unwrap()
            };
            *pte = PageTableEntry::new(frame.ppn, PTEFlags::V | PTEFlags::A | PTEFlags::D | self.flags.into());
            return Ok(frame.clone());
        }
        Err(())
    }

    /// Splits an area with aligned virtual address range.
    ///
    /// Six cases in total:
    /// 1. `start < end <= self.start < self.end` (do nothing)
    /// 2. `self.start < self.end <= start < end` (do nothing)
    /// 3. `start <= self.start < self.end <= end` (whole)
    /// 4. `self.start < start < end < self.end` (three pieces)
    /// 5. `self.start < start < self.end < end` (split right)
    /// 6. `start < self.start < end < self.end` (split left)
    ///
    /// # Argument
    /// - `start`: starting virtual address.
    /// - `end`: ending virtual address.
    ///
    /// # Return
    ///
    /// The first area is:
    /// - the middle part in case 4.
    /// - the right part in case 5.
    /// - the left part in case 6.
    ///
    /// The second area is the third part in case 4.
    pub fn split(&mut self, start: VirtAddr, end: VirtAddr) -> (Option<VMArea>, Option<VMArea>) {
        let start_idx = page_index(self.start_va, start);
        let end_idx = page_index(self.start_va, end);

        if end <= self.start_va
            || self.end_va <= start
            || start <= self.start_va && self.end_va <= end
        {
            (None, None)
        } else if self.start_va < start && end < self.end_va {
            let right_vma = Some(
                Self::new(
                    end,
                    self.end_va,
                    self.flags,
                    self.frames.drain(end_idx..).collect(),
                )
                .unwrap(),
            );
            let mid_vma = Some(
                Self::new(
                    start,
                    end,
                    self.flags,
                    self.frames.drain(start_idx..).collect(),
                )
                .unwrap(),
            );

            self.end_va = start;

            (mid_vma, right_vma)
        } else if self.start_va < start && self.end_va <= end {
            let right_vma = Some(
                Self::new(
                    start,
                    self.end_va,
                    self.flags,
                    self.frames.drain(start_idx..).collect(),
                )
                .unwrap(),
            );

            self.end_va = start;

            (right_vma, None)
        } else if start <= self.start_va && end < self.end_va {
            let left_vma = Some(
                Self::new(
                    self.start_va,
                    end,
                    self.flags,
                    self.frames.drain(..end_idx).collect(),
                )
                .unwrap(),
            );

            self.start_va = end;
            (left_vma, None)
        } else {
            (None, None)
        }
    }

}

/* Derives */
use core::fmt;
impl fmt::Debug for VMArea {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "VMA [0x{:X?}, 0x{:X?}) => {:?}",
            self.start_va.0,
            self.end_va.0,
            self.flags
        )
    }
}