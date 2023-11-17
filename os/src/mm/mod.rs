mod address;
mod frame_allocator;
mod heap_allocator;
mod page_table;
mod ld;
mod kernel;
mod vma;
mod flags;

use self::flags::*;
use vma::VMArea;
use crate::{Result, config::{PAGE_SIZE, USER_HEAP_SIZE, MAX_MAP_COUNT, LOW_MAX_VA}};
use alloc::{vec::Vec, collections::BTreeMap, sync::Arc};
pub use address::{PhysAddr, PhysPageNum, VirtAddr, VirtPageNum};
pub use address::{StepByOne, VPNRange};
pub use frame_allocator::{frame_alloc, frame_alloc_more, frame_dealloc, FrameTracker};
pub use kernel::*;
use page_table::PTEFlags;
pub use page_table::{
    translate_writable_va, translated_byte_buffer, translated_refmut, translated_str, PageTable,
    PageTableEntry, UserBuffer, UserBufferIterator,
};

pub fn init() {
    heap_allocator::init_heap();
    frame_allocator::init_frame_allocator();
    kernel_activate();
}

pub fn init_kernel_space() {
    kernel_activate();
}

/// memory space
pub struct MM {
    /// Holds the pointer to [`PageTable`].
    ///
    /// This object has the ownership of the page table. So the lifetime of [`PageTable`]
    /// depends on the [`MM`] tied to it.
    pub page_table: PageTable,

    /// List of [`VMArea`]s.
    vma_list: Vec<Option<VMArea>>,

    /// Recycled index of `vma_list`.
    vma_recycled: Vec<usize>,

    /// Find an unmapped [`VMArea`] with the target length quickly.
    vma_map: BTreeMap<VirtAddr, usize>,

    /// Last accessed [`VMArea`] cached for faster search with the prediction
    /// of memory locality.
    vma_cache: Option<usize>,

    /// Start virtual address of user code (known as entry point).
    pub entry: VirtAddr,

    /// Start virtual address of heap.
    pub start_brk: VirtAddr,

    /// Heap pointer managed by `sys_brk`.
    pub brk: VirtAddr,
}

/* Global operations */

impl MM {
    /// Create a new empty [`MM`] struct.
    ///
    /// `Trampoline` is mapped to the same code section at first by default.
    /// `Trampoline` is not collected or recorded by VMAs, since this area cannot
    /// be unmapped or modified manually by user. We set the page table flags without
    /// [`PTEFlags::USER_ACCESSIBLE`] so that malicious user cannot jump to this area.
    pub fn new() -> Result<Self> {
        let mut mm = Self {
            page_table: PageTable::new(),
            vma_list: Vec::new(),
            vma_recycled: Vec::new(),
            vma_map: BTreeMap::new(),
            vma_cache: None,
            entry: VirtAddr::from(0),
            start_brk: VirtAddr::from(0),
            brk: VirtAddr::from(0),
        };
        Ok(mm)
    }

    ///
    pub fn token(&self) -> usize {
        self.page_table.token()
    }

    /// Create a new [`MM`] from cloner.
    ///
    /// Uses the copy-on-write technique (COW) to prevent all data of the parent process from being copied
    /// when fork is executed.
    pub fn clone(&mut self) -> Result<Self> {
        let mut page_table = PageTable::new();
        let mut new_vma_list = Vec::new();
        for vma in self.vma_list.iter_mut() {
            if let Some(vma) = vma {
                let mut new_vma = VMArea {
                    flags: vma.flags,
                    start_va: vma.start_va,
                    end_va: vma.end_va,
                    frames: vma.frames.clone(),
                };
                // read-only
                let mut flags = PTEFlags::from(vma.flags);
                flags.remove(PTEFlags::W);

                // map the new vma of child process
                new_vma.map_all(&mut page_table, flags, false)?;
                new_vma_list.push(Some(new_vma));

                // remap the old vma of parent process
                vma.map_all(&mut self.page_table, flags, false)?;
            } else {
                new_vma_list.push(None);
            }
        }

        Ok(Self {
            page_table,
            vma_list: new_vma_list,
            vma_recycled: self.vma_recycled.clone(),
            vma_map: self.vma_map.clone(),
            vma_cache: None,
            entry: self.entry,
            start_brk: self.start_brk,
            brk: self.brk,
        })
    }

    /// A warpper for `translate` in `PageTable`.
    pub fn translate(&mut self, va: VirtAddr) -> Option<PhysAddr> {
        self.page_table
            .translate_va(va)
    }

    /// The number of virtual memory areas.
    pub fn map_count(&mut self) -> usize {
        self.vma_map.len()
    }

    pub fn mmap_min_addr(&self) -> VirtAddr {
        self.start_brk + USER_HEAP_SIZE
    }

    /// Writes to `[start_va, end_va)` using the page table of this address space.
    ///
    /// This function might be terminated if a page in this range is not mapped, thus
    /// the result is unpredictable. So it is marked as `unsafe` for further use.
    ///
    /// The length of `data` may be larger or smaller than the virtual memory range.
    unsafe fn write_vma(
        &mut self,
        data: &[u8],
        start_va: VirtAddr,
        end_va: VirtAddr,
    ) -> Result<()> {
        let end_ptr = data.len();
        let mut data_ptr: usize = 0;
        let mut curr_va = start_va;
        let mut curr_page = VirtPageNum::from(start_va);
        let end_page = VirtPageNum::from(end_va); // inclusive
        loop {
            let page_len: usize = if curr_page == end_page {
                (end_va - curr_va.0).into()
            } else {
                PAGE_SIZE - curr_va.page_offset()
            };

            // Copy data to allocated frames.
            let src = &data[data_ptr..end_ptr.min(data_ptr + page_len)];
            let dst = self.page_table.translate_va(curr_va).and_then(|pa| unsafe {
                Some(core::slice::from_raw_parts_mut(
                    pa.0 as *mut u8,
                    page_len.min(end_ptr - data_ptr),
                ))
            }).expect("dst none");
            dst.copy_from_slice(src);

            // Step to the next page.
            data_ptr += page_len;
            curr_va += page_len;
            curr_page += 1;
            if curr_va >= end_va || data_ptr >= end_ptr {
                break;
            }
        }
        Ok(())
    }

    /// Adds a new [`VMArea`] into the address space.
    ///
    /// This function does not create any memory map for the new area.
    pub fn add_vma(&mut self, vma: VMArea) -> Result<()> {
        if self.map_count() >= MAX_MAP_COUNT {
            log::error!("Too many map areas");
            return Err(());
        }
        let mut index = self.vma_list.len();
        if !self.vma_recycled.is_empty() {
            index = self.vma_recycled.pop().unwrap();
            self.vma_map.insert(vma.start_va, index);
            self.vma_list[index] = Some(vma);
        } else {
            self.vma_map.insert(vma.start_va, index);
            self.vma_list.push(Some(vma));
        }
        self.vma_cache = Some(index);
        Ok(())
    }

    /// Allocates a new [`VMArea`] with the virtual range of `[start_va, end_va)`.
    ///
    /// Writes the data to the mapped physical areas without any check for overlaps.
    ///
    /// This function may be only used when we try to initialize a kernel or user address space.
    pub fn alloc_write_vma(
        &mut self,
        data: Option<&[u8]>,
        start_va: VirtAddr,
        end_va: VirtAddr,
        flags: VMFlags,
    ) -> Result<()> {
        let mut vma = VMArea::new_fixed(start_va, end_va, flags)?;
        vma.map_all(&mut self.page_table, flags.into(), true)?;
        self.add_vma(vma)?;
        if let Some(data) = data {
            unsafe { self.write_vma(data, start_va, end_va)? };
        }
        Ok(())
    }

    /// Allocates a new [`VMArea`].
    ///
    /// # Argument
    /// - `start`: starting virtual address (aligned implicitly)
    /// - `end`: ending virtual address (aligned implicitly)
    /// - `flags`: page table entry flags
    /// - `anywhere`: if set, the given address range will be ignored
    /// - `backend`: if not none, a backend file will be managed by this area
    pub fn alloc_vma(
        &mut self,
        start: VirtAddr,
        end: VirtAddr,
        flags: VMFlags,
        anywhere: bool,
    ) -> Result<VirtAddr> {
        let len = end.0 - start.0;
        let (start, end) = if anywhere {
            let start = self.find_free_area(start, len)?;
            (start, start + len)
        } else {
            do_munmap(self, start, len)?;
            (start, end)
        };
        let vma = VMArea::new_lazy(start, end, flags)?;
        // No need to fllush TLB explicitly; old maps have been cleaned.
        self.add_vma(vma)?;

        Ok(start)
    }

    /// Finds a free area.
    pub fn find_free_area(&self, hint: VirtAddr, len: usize) -> Result<VirtAddr> {
        let mut last_end = VirtAddr::from(0);
        let min_addr = self.mmap_min_addr();
        for (_, index) in self.vma_map.range(hint..) {
            if let Some(vma) = &self.vma_list[*index] {
                if (vma.start_va - last_end.0).0 >= len && vma.start_va - len >= min_addr {
                    return Ok(vma.start_va - len);
                }
                last_end = vma.end_va;
            }
        }
        Err(())
    }

    /// Gets the virtual memory area that contains the virutal address.
    /// Applies the given operation to the target area.
    ///
    /// # Argument
    /// - `va`: virtual address that belongs to the area.
    /// - `op`: a mutable function that receives a mutable reference to the area.
    ///     - `0`: target virtual memory area
    ///     - `1`: page table in this address space
    ///     - `2`: index of the area
    ///
    /// # Error
    /// - [KernelError::PageUnmapped]: the page has not been mapped with `mmap`.
    pub fn get_vma<T>(
        &mut self,
        va: VirtAddr,
        mut op: impl FnMut(&mut VMArea, &mut PageTable, usize) -> Result<T>,
    ) -> Result<T> {
        if let Some(index) = self.vma_cache {
            if let Some(area) = &mut self.vma_list[index] {
                if area.contains(va) {
                    return op(area, &mut self.page_table, index);
                }
            }
        }

        if let Some((_, index)) = self.vma_map.range(..=va).last() {
            if let Some(area) = &mut self.vma_list[*index] {
                if area.contains(va) {
                    self.vma_cache = Some(*index);
                    return op(area, &mut self.page_table, *index);
                }
            }
        }

        Err(())
    }

    /// Gets an ordered vector of the index of virtual memory areas that intersect
    /// with the range.
    pub fn get_vma_range(&mut self, start: VirtAddr, end: VirtAddr) -> Result<Vec<usize>> {
        let mut v = Vec::new();

        // The first area that contains the start of range.
        if let Ok(start_area) = self.get_vma(start, |_, _, index| Ok(index)) {
            v.push(start_area);
        }

        // Find the areas whose starting virtual address is in the given range.
        // These areas must overlap with the given range.
        self.vma_map
            .range(start + 1..end)
            .for_each(|(_, index)| v.push(*index));

        Ok(v)
    }

    /// Allocates a frame for mapped page.
    ///
    /// # Argument
    /// - `va`: starting virtual address.
    pub fn alloc_frame(&mut self, va: VirtAddr) -> Result<Arc<FrameTracker>> {
        self.get_vma(va, |vma, pt, _| {
            vma.alloc_frame(VirtPageNum::from(va), pt)
        })
    }

    /// Allocates a range of frames for given virtual address range [start_va, end_va).
    ///
    /// # Argument
    /// - `start_va`: starting virtual address.
    /// - `end_va`: ending virtual address.
    pub fn alloc_frame_range(
        &mut self,
        start_va: VirtAddr,
        end_va: VirtAddr,
    ) -> Result<Vec<Arc<FrameTracker>>> {
        let mut frames = Vec::new();
        for vpn in VPNRange::new(start_va.into(), end_va.into()) {
            frames.push(
                self.get_vma(VirtAddr::from(vpn), |vma, pt, _| vma.alloc_frame(vpn, pt))?,
            );
        }
        Ok(frames)
    }

    /// Allocates a type starting from the given virtual address.
    ///
    /// # Argument
    /// - `va`: starting virtual address where the data type locates.
    pub fn alloc_type<T: Sized>(&mut self, va: VirtAddr) -> Result<()> {
        self.alloc_frame_range(va, va + core::mem::size_of::<T>())?;
        Ok(())
    }

    /// Allocates a type and writes data to the physical address.
    ///
    /// # Argument
    /// - `va`: starting virtual address where the data type locates.
    /// - `data`: reference of data type.
    pub fn alloc_write_type<T: Sized>(&mut self, va: VirtAddr, data: &T) -> Result<()> {
        let size = core::mem::size_of::<T>();
        let end_va = va + size;
        self.alloc_frame_range(va, end_va)?;
        let data = unsafe { core::slice::from_raw_parts(data as *const T as *const _, size) };
        unsafe { self.write_vma(data, va, end_va)? };
        Ok(())
    }

    /// Gets bytes translated with the range of [start_va, start_va + len),
    /// which might cover several pages.
    ///
    /// The buffer may not be allocated with frames, so new frames will be
    /// allocated for further modifications on this buffer.
    ///
    /// # Argument
    /// - `va`: starting virtual address
    /// - `len`: total length of the buffer
    pub fn get_buf_mut(&mut self, va: VirtAddr, len: usize) -> Result<UserBuffer> {
        let mut start_va = va;
        let end_va = start_va + len;
        let mut v = Vec::new();
        while start_va < end_va {
            let next_page = VirtPageNum::from(start_va) + 1;
            let page_off = start_va.page_offset();
            let page_len: usize = (end_va - start_va.0).0
                .min((VirtAddr::from(next_page) - start_va.0).0);
            let frame = self.alloc_frame(start_va)?;
            v.push(&mut frame.ppn.get_bytes_array()[page_off..page_off + page_len]);
            start_va += page_len;
        }
        Ok(UserBuffer::new(v))
    }

}

use core::fmt;
impl fmt::Debug for MM {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(
            f,
            "\ntoken: 0x{:X?}\nAddress Space: entry=0x{:X?}, start_brk=0x{:X?}, brk=0x{:X?}",
            self.page_table.token(),
            self.entry.0,
            self.start_brk.0,
            self.brk.0,
        )?;
        for (_, index) in &self.vma_map {
            if let Some(vma) = &self.vma_list[*index] {
                writeln!(f, "{:#?}", vma)?;
            }
        }
        Ok(())
    }
}



/// Value aligned to the multiple of page size.
pub fn page_align(value: usize) -> usize {
    value & !(PAGE_SIZE - 1)
}

/// Page index from start in a range of pages
pub fn page_index(start_va: VirtAddr, va: VirtAddr) -> usize {
    va.floor().0 - start_va.floor().0
}

/// The number of total pages covered by this exclusive range.
pub fn page_count(start_va: VirtAddr, end_va: VirtAddr) -> usize {
    end_va.floor().0 - start_va.floor().0 + 1
}


/// A helper for [`syscall_interface::SyscallProc::munmap`].
pub fn do_munmap(mm: &mut MM, start: VirtAddr, len: usize) -> Result<()> {
    let len = page_align(len);
    if !start.aligned() || len == 0 {
        log::error!("munmap invalid args");
        return Err(());
    }
    let end = start + len;

    // avoid crashes
    mm.vma_cache = None;

    let vma_range = mm.get_vma_range(start, end)?;
    for index in vma_range {
        let mut need_remove = false;
        let vma = mm.vma_list[index].as_mut().unwrap();
        let mut new_vma = None;

        if start > vma.start_va && end < vma.end_va && mm.vma_map.len() >= MAX_MAP_COUNT {
            log::error!("Too many map areas");
            return Err(());
        }

        // intersection cases
        if vma.start_va >= start && vma.end_va <= end {
            vma.unmap_all(&mut mm.page_table).unwrap();
            need_remove = true;
        } else if vma.start_va < start && vma.end_va > end {
            let (mid, right) = vma.split(start, end);
            mid.unwrap().unmap_all(&mut mm.page_table).unwrap();
            new_vma = right;
        } else if vma.end_va > end {
            // vma starting address modified to end
            mm.vma_map.remove(&vma.start_va);
            let (left, _) = vma.split(start, end);
            mm.vma_map.insert(vma.start_va, index);
            left.unwrap().unmap_all(&mut mm.page_table).unwrap();
        } else {
            let (right, _) = vma.split(start, end);
            right.unwrap().unmap_all(&mut mm.page_table).unwrap();
        }

        if need_remove {
            let vma = mm.vma_list[index].take().unwrap();
            mm.vma_recycled.push(index);
            mm.vma_map.remove(&vma.start_va);
        }

        if let Some(new_vma) = new_vma {
            mm.add_vma(new_vma).unwrap();
        }
    }
    Ok(())
}

/// A helper for [`syscall_interface::SyscallProc::mmap`].
///
/// TODO: MAP_SHARED and MAP_PRIVATE
pub fn do_mmap(
    mm: &mut MM,
    hint: VirtAddr,
    len: usize,
    prot: MmapProt,
    flags: MmapFlags,
    fd: usize,
    off: usize,
) -> Result<usize> {
    log::trace!(
        "MMAP [{:?}, {:?}) {:#?} {:#?} 0x{:X} 0x{:X}",
        hint,
        hint + len,
        prot,
        flags,
        fd,
        off
    );

    if len == 0
        || !hint.aligned()
        || !(hint + len).aligned()
        || hint + len > VirtAddr::from(LOW_MAX_VA)
        || hint == VirtAddr::from(0) && flags.contains(MmapFlags::MAP_FIXED)
    {
        log::error!("mmap invalid args");
        return Err(());
    }

    if mm.map_count() >= MAX_MAP_COUNT {
        log::error!("Too many map areas");
        return Err(());
    }

    // Find an available area by kernel.
    let anywhere = (hint == VirtAddr::from(0)) && !flags.contains(MmapFlags::MAP_FIXED);

    // Handle different cases indicated by `MmapFlags`.
    if flags.contains(MmapFlags::MAP_ANONYMOUS) {
        if fd as isize == -1 && off == 0 {
            if let Ok(start) = mm.alloc_vma(hint, hint + len, prot.into(), anywhere) {
                return Ok(start.0);
            } else {
                log::error!("no memory");
                return Err(());
            }
        }
        log::error!("mmap invalid args");
        return Err(());
    }

    // TODO: Map to backend file.
    

    // Invalid arguments or unimplemented cases
    // flags contained none of MAP_PRIVATE, MAP_SHARED, or MAP_SHARED_VALIDATE.
    // Err(Errno::EINVAL)
    log::error!("mmap invalid args");
    return Err(());
}