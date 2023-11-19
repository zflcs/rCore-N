//! This mod provides functions to link a shared object file with the memory set


use xmas_elf::dynamic::Tag;
use xmas_elf::program::Type::Load;
use xmas_elf::sections::SectionData;
use xmas_elf::sections::SectionData::{DynSymbolTable64, Dynamic64, Undefined};
use xmas_elf::symbol_table::DynEntry64;
use xmas_elf::symbol_table::Entry;
use xmas_elf::{header, ElfFile};
use xmas_elf::{sections, program};
use xmas_elf::program::SegmentData;
use super::{MmapFlags, VirtAddr, VirtPageNum, VMFlags, MM};
use crate::Result;
use crate::lkm::api::user_rt;
use crate::lkm::{const_reloc, LKM_MANAGER};
///
/// Create address space from elf.
pub fn from_elf(elf_data: &[u8], mm: &mut MM) -> Result<()> {
    
    let elf = ElfFile::new(elf_data).unwrap();
    let exported_symbols = user_rt(&elf);
    mm.exported_symbols = exported_symbols;
    let elf_hdr = elf.header;

    // Check elf type
    if (elf_hdr.pt2.type_().as_type() != header::Type::Executable
        && elf_hdr.pt2.type_().as_type() != header::Type::SharedObject)
        // 64-bit format
        || elf_hdr.pt1.class() != header::Class::SixtyFour
        // 'E', 'L', 'F'
        || elf_hdr.pt1.magic != [0x7f, 0x45, 0x4c, 0x46]
        // RISC-V
        || elf_hdr.pt2.machine().as_machine() != header::Machine::RISC_V
    {
        return Err(());
    }

    // Load program header
    let mut max_vpn: VirtPageNum = VirtPageNum::from(0);
    for phdr in elf.program_iter() {
        match phdr.get_type().unwrap() {
            Load => {
                let start_va: VirtAddr = (phdr.virtual_addr() as usize).into();
                let end_va: VirtAddr = ((phdr.virtual_addr() + phdr.mem_size()) as usize).into();
                max_vpn = end_va.ceil();

                // Map flags
                let mut map_flags = VMFlags::USER;
                let phdr_flags = phdr.flags();
                if phdr_flags.is_read() {
                    map_flags |= VMFlags::READ;
                }
                if phdr_flags.is_write() {
                    map_flags |= VMFlags::WRITE;
                }
                if phdr_flags.is_execute() {
                    map_flags |= VMFlags::EXEC;
                }
                // log::debug!("{:X?}-{:X?}", start_va, end_va);

                // Allocate a new virtual memory area
                let data = match phdr.get_data(&elf).unwrap() {
                    SegmentData::Undefined(data) => data,
                    _ => return Err(()),
                };
                let start = start_va.floor();
                let end = end_va.ceil();
                // Address may not be aligned.
                mm.alloc_write_vma(
                    Some(data),
                    start.into(),
                    end.into(),
                    map_flags,
                )?;
            }
            _ => {}
        };
    }
    // .rela.dyn

    // .rela.plt

    // Set brk location
    mm.start_brk = VirtAddr::from(max_vpn);
    mm.brk = mm.start_brk;

    // Set user entry
    mm.entry = VirtAddr::from(elf_hdr.pt2.entry_point() as usize);
    LKM_MANAGER.lock().link_module("libsharedscheduler.so", mm, &elf);
    
    
    Ok(())
}

