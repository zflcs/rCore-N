pub mod flags;
mod init;

use core::{arch::global_asm, sync::atomic::{AtomicUsize, Ordering}};

use alloc::{collections::BTreeMap, string::String, vec::Vec};
use executor::MAX_PRIO;
use log::info;
use vdso::so_table;
use xmas_elf::{
    header,
    program::{self, SegmentData},
    ElfFile,
};

use crate::{
    arch::mm::{Page, VirtAddr, PAGE_SIZE},
    config::{ADDR_ALIGN, ELF_BASE_RELOCATE, USER_STACK_BASE, USER_STACK_SIZE, HEAP_POINTER, PRIO_POINTER},
    error::{KernelError, KernelResult},
    mm::{VMFlags, MM},
    lkm::LKM_MANAGER,
};

use self::{
    flags::AuxType,
    init::{InitInfo, InitStack},
};

// /// Finds the user ELF in the given directory and creates the task.
// pub fn from_args(dir: String, args: Vec<String>) -> KernelResult<Arc<Task>> {
//     if args.len() < 1 {
//         return Err(KernelError::InvalidArgs);
//     }
//     let name = args[0].as_str();
//     let path = dir.clone() + "/" + name;
//     let file = unsafe {
//         open(Path::from(path), OpenFlags::O_RDONLY)
//             .map_err(|errno| KernelError::Errno(errno))?
//             .read_all()
//     };
//     Ok(Arc::new(Task::new(dir, file.as_slice(), args)?))
// }

/// Create address space from elf.
pub fn from_elf(elf_data: &[u8], args: Vec<String>, mm: &mut MM) -> KernelResult<VirtAddr> {
    let elf = ElfFile::new(elf_data).unwrap();
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
        return Err(KernelError::ELFInvalidHeader);
    }

    // Dynamic address
    let mut dyn_base = 0;
    let elf_base_va = if let Some(phdr) = elf
        .program_iter()
        .find(|phdr| phdr.get_type() == Ok(program::Type::Load) && phdr.offset() == 0)
    {
        let phdr_va = phdr.virtual_addr() as usize;
        if phdr_va != 0 {
            phdr_va
        } else {
            // If the first segment starts at 0, we need to put it at a higher address
            // to avoid conflicts with user programs.
            dyn_base = ELF_BASE_RELOCATE;
            ELF_BASE_RELOCATE
        }
    } else {
        0
    };

    // Load program header
    let mut max_page = Page::from(0);
    for phdr in elf.program_iter() {
        match phdr.get_type().unwrap() {
            program::Type::Load => {
                let start_va: VirtAddr = (phdr.virtual_addr() as usize).into();
                let start_aligned_va = start_va.page_align();
                let end_va: VirtAddr = ((phdr.virtual_addr() + phdr.mem_size()) as usize + PAGE_SIZE - 1).into();
                let end_aligned_va = end_va.page_align();
                max_page = Page::floor(end_aligned_va - 1) + 1;

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

                // Allocate a new virtual memory area
                let data = match phdr.get_data(&elf).unwrap() {
                    SegmentData::Undefined(data) => data,
                    _ => return Err(KernelError::ELFInvalidSegment),
                };
                
                // Address may not be aligned.
                mm.alloc_write_vma(
                    Some(data),
                    start_aligned_va + dyn_base,
                    end_aligned_va + dyn_base,
                    map_flags,
                )?;
            }
            program::Type::Interp => {
                // let data = match phdr.get_data(&elf).unwrap() {
                //     SegmentData::Undefined(data) => data,
                //     _ => return Err(KernelError::ELFInvalidSegment),
                // };
                // let path = unsafe {raw_ptr_to}
            }
            _ => {}
        };
    }

    // .rela.dyn

    // .rela.plt

    // Set brk location
    mm.start_brk = max_page.start_address() + dyn_base;
    mm.brk = mm.start_brk;
    // link sharedscheduler
    LKM_MANAGER.lock().as_mut().unwrap().link_module("sharedscheduler", mm, Some(so_table(&elf)))?;
    // recore lockheap into HEAP_POINTER
    let heap_addr = elf.find_section_by_name(".data").unwrap().address() as usize;
    mm.alloc_write_vma(None, HEAP_POINTER.into(), (HEAP_POINTER + PAGE_SIZE).into(), VMFlags::READ | VMFlags::WRITE | VMFlags::USER)?;
    let paddr = mm.translate(HEAP_POINTER.into())?;
    unsafe { *(paddr.value() as *mut usize) = heap_addr; }
    let prio_ptr = mm.translate(PRIO_POINTER.into())?.value() as *mut AtomicUsize;
    (unsafe { &*prio_ptr }).store(MAX_PRIO - 1, Ordering::Relaxed);

    // // set Global bitmap
    // extern "C" { fn sshared(); }
    // let _ = mm.page_table.map(Page::from(GLOBAL_BITMAP_BASE), Frame::from(sshared as usize), PTEFlags::READABLE | PTEFlags::USER_ACCESSIBLE | PTEFlags::VALID | PTEFlags::ACCESSED | PTEFlags::DIRTY);
    // let (_, pte) = mm.page_table.walk(Page::from(GLOBAL_BITMAP_BASE)).unwrap();
    // log::trace!("map {:?}", pte);
    // Set user entry
    // mm.entry = VirtAddr::from(elf_hdr.pt2.entry_point() as usize) + dyn_base;
    mm.entry = LKM_MANAGER.lock().as_mut().unwrap().resolve_symbol("user_entry").unwrap().into();

    // Initialize user stack
    let ustack_base = USER_STACK_BASE - ADDR_ALIGN;
    let ustack_top = USER_STACK_BASE - USER_STACK_SIZE;
    mm.alloc_write_vma(
        None,
        ustack_top.into(),
        ustack_base.into(),
        VMFlags::READ | VMFlags::WRITE | VMFlags::USER,
    )?;
    let mut vsp = VirtAddr::from(ustack_base);
    let sp = mm.translate(vsp)?;
    let init_stack = InitStack::serialize(
        InitInfo {
            args,
            // TODO
            envs: Vec::new(),
            auxv: {
                let mut at_table = BTreeMap::new();
                at_table.insert(
                    AuxType::AT_PHDR,
                    elf_base_va + elf_hdr.pt2.ph_offset() as usize,
                );
                at_table.insert(AuxType::AT_PHENT, elf_hdr.pt2.ph_entry_size() as usize);
                at_table.insert(AuxType::AT_PHNUM, elf_hdr.pt2.ph_count() as usize);
                at_table.insert(AuxType::AT_RANDOM, 0);
                at_table.insert(AuxType::AT_PAGESZ, PAGE_SIZE);
                at_table
            },
        },
        sp,
        vsp,
    );
    vsp -= init_stack.len();
    Ok(vsp)
}


use lazy_static::*;
global_asm!(include_str!("link_app.asm"));


pub fn get_num_app() -> usize {
    extern "C" {
        fn _num_app();
    }
    unsafe { (_num_app as usize as *const usize).read_volatile() }
}

pub fn get_app_data(app_id: usize) -> &'static [u8] {
    extern "C" {
        fn _num_app();
    }
    let num_app_ptr = _num_app as usize as *const usize;
    let num_app = get_num_app();
    let app_start = unsafe { core::slice::from_raw_parts(num_app_ptr.add(1), num_app + 1) };
    assert!(app_id < num_app);
    unsafe {
        core::slice::from_raw_parts(
            app_start[app_id] as *const u8,
            app_start[app_id + 1] - app_start[app_id],
        )
    }
}

lazy_static! {
    static ref APP_NAMES: Vec<&'static str> = {
        let num_app = get_num_app();
        extern "C" {
            fn _app_names();
        }
        let mut start = _app_names as usize as *const u8;
        let mut v = Vec::new();
        unsafe {
            for _ in 0..num_app {
                let mut end = start;
                while end.read_volatile() != b'\0' {
                    end = end.add(1);
                }
                let slice = core::slice::from_raw_parts(start, end as usize - start as usize);
                let str = core::str::from_utf8(slice).unwrap();
                v.push(str);
                start = end.add(1);
            }
        }
        v
    };
}

#[allow(unused)]
pub fn get_app_data_by_name(name: &str) -> Option<&'static [u8]> {
    let num_app = get_num_app();
    (0..num_app)
        .find(|&i| APP_NAMES[i] == name)
        .map(get_app_data)
}

pub fn list_apps() {
    info!("/**** APPS ****");
    for app in APP_NAMES.iter() {
        info!("{}", app);
    }
    info!("**************/")
}
