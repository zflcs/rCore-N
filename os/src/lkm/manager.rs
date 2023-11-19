use super::api::*;
use super::const_reloc as loader;
use super::structs::*;
use spin::{Lazy, Mutex};
use alloc::collections::BTreeMap;
use alloc::string::*;
use alloc::sync::Arc;
use alloc::boxed::Box;
use alloc::vec::*;
use xmas_elf::header::Data;
use core::iter::zip;
use core::mem::transmute;
use crate::fs::OpenFlags;
use crate::fs::open_file;
use crate::mm::*;
use crate::{Result, config::PAGE_SIZE};
use xmas_elf::dynamic::Tag;
use xmas_elf::program::Type::Load;
use xmas_elf::sections::SectionData;
use xmas_elf::sections::SectionData::{DynSymbolTable64, Dynamic64, Undefined};
use xmas_elf::symbol_table::DynEntry64;
use xmas_elf::symbol_table::Entry;
use xmas_elf::{header, ElfFile};
use xmas_elf::program::SegmentData;
use crate::mm::*;

/// Module Manager is the core part of LKM.
pub struct ModuleManager {
    loaded_modules: Vec<Box<LoadedModule>>,
}

pub static LKM_MANAGER: Lazy<Mutex<ModuleManager>> = Lazy::new(|| {
    let mut kmm = ModuleManager::new();
    info!("[LKM] Loadable Kernel Module Manager loading...");
    kmm.init_module("libsharedscheduler.so").unwrap();
    info!("[LKM] Loadable Kernel Module Manager loaded!");
    Mutex::new(kmm)
});


impl ModuleManager {
    pub fn new() -> Self {
        Self {
            loaded_modules: Vec::new(),
        }
    }

    pub fn resolve_symbol(&self, symbol: &str) -> Option<usize> {
        self.find_symbol_in_deps(symbol, 0)
    }

    fn find_symbol_in_deps(&self, symbol: &str, this_module: usize) -> Option<usize> {
        for km in self.loaded_modules.iter().rev() {
            for sym in km.exported_symbols.iter() {
                if (&sym.name) == symbol {
                    return Some(sym.loc);
                }
            }
        }
        None
    }
    fn get_symbol_loc(
        &self,
        mm: &mut MM,
        symbol_index: usize,
        elf: &ElfFile,
        dynsym: &[DynEntry64],
        base: usize,
        find_dependency: bool,
        this_module: usize,
    ) -> Option<usize> {
        // info!("symbol index: {}", symbol_index);
        if symbol_index == 0 {
            return Some(0);
        }
        let selected_symbol = &dynsym[symbol_index];
        if selected_symbol.shndx() == 0 {
            if find_dependency {
                // info!("symbol name: {}", selected_symbol.get_name(elf).unwrap());
                for sym in mm.exported_symbols.iter() {
                    if sym.0 == selected_symbol.get_name(elf).unwrap() {
                        return Some(sym.1.loc)
                    }
                }
                self.find_symbol_in_deps(selected_symbol.get_name(elf).unwrap(), this_module)
            } else {
                None
            }
        } else {
            Some(base + (selected_symbol.value() as usize))
        }
    }

    pub fn init_module(&mut self, module_name: &str) -> Result<()> {
        for i in 0..self.loaded_modules.len() {
            if &self.loaded_modules[i].info.name == module_name {
                error!(
                    "[LKM] another instance of module {} has been loaded!",
                    self.loaded_modules[i].info.name
                );
                return Err(());
            }
        }
        let module_content = open_file(module_name, OpenFlags::RDONLY).unwrap().read_all();
        let elf = ElfFile::new(&module_content).expect("[LKM] failed to read elf");
        match elf.header.pt2 {
            header::HeaderPt2::Header32(_) => {
                error!("[LKM] 32-bit elf is not supported!");
                return Err(());
            },
            _ => {},
        };
        match elf.header.pt2.type_().as_type() {
            header::Type::Executable => {
                error!("[LKM] a kernel module must be some shared object!");
                return Err(());
            }
            header::Type::SharedObject => {}
            _ => {
                error!("[LKM] ELF is not executable or shared object");
                return Err(());
            }
        }
        let module_info = elf.find_section_by_name(".module_info").ok_or_else(|| {
            error!("[LKM] module_info metadata not found!");
            ()
        })?;
        let minfo = if let Undefined(info_content) = module_info.get_data(&elf).map_err(|_| {
            error!("[LKM] load module_info error!");
            ()
        })? {
            ModuleInfo::parse(core::str::from_utf8(info_content).unwrap()).ok_or_else(
                || {
                    error!("[LKM] parse info error!");
                    ()
                },
            )?
        } else {
            return Err(());
        };
        info!(
            "[LKM] loading module {} version {} api_version {}, exported {:?}",
            minfo.name, minfo.version, minfo.api_version, minfo.exported_symbols
        );

        let mut max_addr = VirtAddr::from(0);
        let mut min_addr = VirtAddr::from(usize::MAX);
        for ph in elf.program_iter() {
            if ph.get_type().unwrap() == Load {
                if (ph.virtual_addr() as usize) < min_addr.0 {
                    min_addr = (ph.virtual_addr() as usize).into();
                }
                if (ph.virtual_addr() + ph.mem_size()) as usize > max_addr.0 {
                    max_addr = ((ph.virtual_addr() + ph.mem_size()) as usize).into();
                }
            }
        }
        let map_len = (max_addr.ceil() - min_addr.floor().0).0 * PAGE_SIZE;
        // We first map a huge piece. This requires the kernel model to be dense and not abusing vaddr.
        let base = KERNEL_SPACE.lock().find_free_area(min_addr - min_addr.page_offset(), map_len)?.0;
        let vspace = (base, map_len);
        for ph in elf.program_iter() {
            if ph.get_type().map_err(|_| {
                error!("[LKM] program header error!");
                ()
            })? == Load
            {
                let prog_start_addr = base + (ph.virtual_addr() as usize);
                let prog_end_addr = prog_start_addr + (ph.mem_size() as usize);
                let offset = ph.offset() as usize;
                let flags = ph.flags();
                let mut attr = VMFlags::empty();
                if flags.is_write() {
                    attr |= VMFlags::WRITE;
                }
                if flags.is_execute() {
                    attr |= VMFlags::EXEC;
                }
                if flags.is_read() {
                    attr |= VMFlags::READ;
                }
                // Allocate a new virtual memory area
                let data = &elf.input[ph.offset() as usize..(ph.offset() + ph.file_size()) as usize];
                let start = VirtAddr::from(prog_start_addr).floor();
                let end = VirtAddr::from(prog_end_addr).ceil();
                // log::debug!("alloc so vma {:#X?}-{:#X?}", prog_start_addr - base, prog_end_addr - base);
                KERNEL_SPACE.lock().alloc_write_vma(
                    Some(data),
                    start.into(), 
                    end.into(),
                    attr, 
                );
            }
        }

        let mut loaded_minfo = Box::new(LoadedModule {
            info: minfo,
            exported_symbols: Vec::new(),
            used_counts: 0,
            using_counts: Arc::new(ModuleRef {}),
            vspace,
            lock: Mutex::new(()),
            state: ModuleState::Ready,
            depend_symbols: Vec::new(),
        });
        info!(
            "[LKM] module load done at 0x{:X?}, now need to do the relocation job.",
            base
        );
        // We only search two tables for relocation info: the symbols from itself, and the symbols from the global exported symbols.
        let dynsym_table = dynsym_table(&elf);
        info!("[LKM] Loading dynamic entry");
        let dynamic_entries = dynamic_table(&elf);
        // info!("[LKM] Iterating modules");
        // start, total_size, single_size
        let mut reloc_jmprel: (usize, usize, usize) = (0, 0, 0);
        let mut reloc_rel: (usize, usize, usize) = (0, 0, 16);
        let mut reloc_rela: (usize, usize, usize) = (0, 0, 24);
        for dent in dynamic_entries.iter() {
            // log::debug!("{:?}", dent.get_tag());
            match dent.get_tag().map_err(|_| {
                error! {"[LKM] invalid dynamic entry!"};
                ()
            })? {
                Tag::JmpRel => {
                    reloc_jmprel.0 = dent.get_ptr().unwrap() as usize;
                }
                Tag::PltRelSize => {
                    reloc_jmprel.1 = dent.get_val().unwrap() as usize;
                }
                Tag::PltRel => {
                    reloc_jmprel.2 = if (dent.get_val().unwrap()) == 7 {
                        24
                    } else {
                        16
                    }
                }
                Tag::Rel => {
                    reloc_rel.0 = dent.get_ptr().unwrap() as usize;
                }
                Tag::RelSize => {
                    reloc_rel.1 = dent.get_val().unwrap() as usize;
                }
                Tag::Rela => {
                    reloc_rela.0 = dent.get_ptr().unwrap() as usize;
                }
                Tag::RelaSize => {
                    reloc_rela.1 = dent.get_val().unwrap() as usize;
                }
                _ => {}
            }
        }
        // info!("[LKM] relocating three sections");
        let this_module = &(*loaded_minfo) as *const _ as usize;
        let mut kernel_mm = KERNEL_SPACE.lock();
        self.reloc_symbols(&mut* kernel_mm, &elf, reloc_jmprel, base, dynsym_table, this_module);
        self.reloc_symbols(&mut* kernel_mm, &elf, reloc_rel, base, dynsym_table, this_module);
        self.reloc_symbols(&mut* kernel_mm, &elf, reloc_rela, base, dynsym_table, this_module);
        info!("[LKM] relocation done. adding module to manager and call init_module");
        for exported in loaded_minfo.info.exported_symbols.iter() {
            for sym in dynsym_table.iter() {
                if exported
                    == sym.get_name(&elf).map_err(|_| {
                        error!("[LKM] load symbol name error!");
                        ()
                    })?
                {
                    // log::debug!("exported symbols {}", exported);
                    let exported_symbol = ModuleSymbol {
                        name: exported.clone(),
                        loc: base + (sym.value() as usize),
                    };
                    loaded_minfo.exported_symbols.push(exported_symbol);
                }
            }
        }
        self.loaded_modules.push(loaded_minfo);
        Ok(())
    }

    fn relocate_single_symbol(
        &mut self,
        mm: &mut MM,
        base: usize,
        reloc_addr: usize,
        addend: usize,
        sti: usize,
        itype: usize,
        elf: &ElfFile,
        dynsym: &[DynEntry64],
        this_module: usize,
    ) {
        // info!("Resolving symbol {} reloc_addr {:#X?} addend {:#X?} itype {}", sti, reloc_addr, addend, itype);
        let sym_val = self
            .get_symbol_loc(mm, sti, elf, dynsym, base, true, this_module);
        if sym_val.is_none() {
            error!("[LKM] resolve symbol failed!");
            return;
        }
        let sym_val = sym_val.unwrap();
        // log::debug!("{:X?}-{:X?}", base, reloc_addr);
        match itype as usize {
            loader::REL_NONE => {}
            loader::REL_OFFSET32 => {
                panic!("[LKM] REL_OFFSET32 detected!")
                //    addend-=reloc_addr;
            }
            loader::REL_SYMBOLIC => unsafe {
                write_to_addr(mm, base, reloc_addr, sym_val + addend);
            },
            loader::REL_GOT => unsafe {
                write_to_addr(mm, base, reloc_addr, sym_val + addend);
            },
            loader::REL_PLT => unsafe {
                write_to_addr(mm, base, reloc_addr, sym_val + addend);
            },
            loader::REL_RELATIVE => unsafe {
                write_to_addr(mm, base, reloc_addr, base + addend);
            },
            _ => {
                panic!("[LKM] unsupported relocation type: {}", itype);
            }
        }
    }
    fn reloc_symbols(
        &mut self,
        mm: &mut MM,
        elf: &ElfFile,
        (start, total_size, _single_size): (usize, usize, usize),
        base: usize,
        dynsym: &[DynEntry64],
        this_module: usize,
    ) {
        if total_size == 0 {
            return;
        }
        // log::debug!("{:#X?}-{:#X?}-{:#X?}", start, total_size, _single_size);
        for s in elf.section_iter() {
            if (s.offset() as usize) == start {
                {
                    match s.get_data(elf).unwrap() {
                        SectionData::Rela64(rela_items) => {
                            for item in rela_items.iter() {
                                let addend = item.get_addend() as usize;
                                let reloc_addr = item.get_offset() as usize;
                                let sti = item.get_symbol_table_index() as usize;
                                let itype = item.get_type() as usize;
                                self.relocate_single_symbol(
                                    mm,
                                    base,
                                    reloc_addr,
                                    addend,
                                    sti,
                                    itype,
                                    elf,
                                    dynsym,
                                    this_module,
                                );
                            }
                        }
                        SectionData::Rel64(rel_items) => {
                            for item in rel_items.iter() {
                                let addend = 0 as usize;
                                let reloc_addr = item.get_offset() as usize;
                                let sti = item.get_symbol_table_index() as usize;
                                let itype = item.get_type() as usize;
                                self.relocate_single_symbol(
                                    mm,
                                    base,
                                    reloc_addr,
                                    addend,
                                    sti,
                                    itype,
                                    elf,
                                    dynsym,
                                    this_module,
                                );
                            }
                        }
                        _ => {
                            panic!("[LKM] bad relocation section type!");
                        }
                    }
                }
                break;
            }
        }
    }
    pub fn delete_module(&mut self, name: &str, _flags: u32) -> Result<()> {
        //unimplemented!("[LKM] You can't plug out what's INSIDE you, RIGHT?");

        info!("[LKM] now you can plug out a kernel module!");
        let mut found = false;
        for i in 0..self.loaded_modules.len() {
            if &(self.loaded_modules[i].info.name) == name {
                let mut current_module = &mut (self.loaded_modules[i]);
                let mod_lock = current_module.lock.lock();
                if current_module.used_counts > 0 {
                    error!("[LKM] some module depends on this module!");
                    return Err(());
                }
                if Arc::strong_count(&current_module.using_counts) > 0 {
                    error!("[LKM] there are references to the module!");
                    return Err(());
                }
                let mut cleanup_func: usize = 0;
                for entry in current_module.exported_symbols.iter() {
                    if (&(entry.name)) == "cleanup_module" {
                        cleanup_func = entry.loc;
                        break;
                    }
                }
                if cleanup_func > 0 {
                    unsafe {
                        current_module.state = ModuleState::Unloading;
                        let cleanup_module: fn() = transmute(cleanup_func);
                        (cleanup_module)();
                    }
                } else {
                    error!("[LKM] you cannot plug this module out.");
                    return Err(());
                }
                drop(mod_lock);

                let _my_box = self.loaded_modules.remove(i);
                unsafe {
                    LKM_MANAGER.force_unlock();
                }
                //drop(mod_lock);
                found = true;
                break;
            }
        }
        if found {
            Ok(())
        } else {
            Err(())
        }
    }

    /// link the mm with the target module
    pub fn link_module(&mut self, module_name: &str, mm: &mut MM, elf: &ElfFile) -> Result<()> {
        // search the target module
        let mut module_space: Option<(usize, usize)> = None;
        for i in 0..self.loaded_modules.len() {
            if self.loaded_modules[i].info.name == module_name {
                module_space = Some(self.loaded_modules[i].vspace);
                break;
            }
        }
        match module_space {
            None => {
                error!("[LKM] {} module is not existed!", module_name);
                return Err(());
            }
            Some((start, len)) => {
                let base = mm.find_free_area(VirtAddr(0), len).unwrap();
                let vmas = KERNEL_SPACE.lock().get_vma_range(start.into(), (start + len).into()).unwrap();
                let mut vmas = Vec::new();

                // The target vmas.
                let mut curr_start = VirtAddr(start);
                while curr_start.0 < start + len {
                    if let Ok(vma) = KERNEL_SPACE.lock().get_vma(curr_start, |vma, _, _| 
                        Ok((vma.flags, vma.start_va, vma.end_va))) 
                    {
                        curr_start = vma.2;
                        vmas.push(vma);
                    }
                }
                let mut new_va_start = base;
                let mut new_va_end = base;
                for (flags, start_va, end_va) in vmas {
                    // log::debug!("{:?} {:X?} {:X?}", flags, start_va, end_va);
                    new_va_end += end_va.0 - start_va.0;
                    // the ppn_range in kernel space
                    let ppn_start = KERNEL_SPACE.lock().translate_pte(start_va.into()).unwrap().ppn();
                    let ppn_end = KERNEL_SPACE.lock().translate_pte((end_va - PAGE_SIZE).into()).unwrap().ppn();
                    // log::debug!("{:X?} {:X?}", ppn_start, ppn_end);
                    let ppn_range = PPNRange::new(ppn_start, ppn_end);
                    // the write page need to alloc to new position
                    if flags.contains(VMFlags::WRITE) {
                        let src_ptr = KERNEL_SPACE.lock().translate(start_va).unwrap().0;
                        let len = end_va.0 - start_va.0;
                        // log::debug!("src {:X?}, len {}", src_ptr, len);
                        let src_slice = unsafe { core::slice::from_raw_parts(src_ptr as _, len) };
                        // log::debug!("{:X?}", &src_slice[0x12ca..0x12ca + 16]);
                        let end = new_va_start.0 + end_va.0 - start_va.0;
                        mm.alloc_write_vma(
                            Some(src_slice), 
                            new_va_start, 
                            end.into(), 
                            flags | VMFlags::USER
                        ).unwrap();
                    } else {    // the read-only vma can shared
                        // the vpn_range in mm 
                        let end = new_va_start.0 + end_va.0 - start_va.0;
                        let vpn_range = VPNRange::new(
                            new_va_start.into(), 
                            VirtAddr(end).into()
                        );
                        for (vpn, ppn) in zip(vpn_range, ppn_range) {
                            // log::debug!("{:X?}-{:X?}", vpn, ppn);
                            mm.page_table.map(vpn, ppn, (flags| VMFlags::USER).into());
                        }
                        let src_ptr = mm.translate(new_va_start).unwrap().0;
                        let src_slice = unsafe { core::slice::from_raw_parts(src_ptr as *const u8, len) };
                        // log::debug!("{:X?}", &src_slice[0x12ca..0x12ca + 16]);
                        let vma = VMArea::new(
                            vpn_range.get_start().into(),
                            vpn_range.get_end().into(),
                            flags | VMFlags::USER,
                            Vec::new()
                        ).unwrap();
                        mm.add_vma(vma);
                    }
                    new_va_start = new_va_end;
                }
                // We only search two tables for relocation info: the symbols from itself, and the symbols from the global exported symbols.
                let module_content = open_file(module_name, OpenFlags::RDONLY).unwrap().read_all();
                let elf = ElfFile::new(&module_content).expect("[LKM] failed to read elf");
                let dynsym_table = dynsym_table(&elf);
                // info!("[LKM] Loading dynamic entry");
                let dynamic_entries = dynamic_table(&elf);
                // info!("[LKM] Iterating modules");
                // start, total_size, single_size
                let mut reloc_jmprel: (usize, usize, usize) = (0, 0, 0);
                let mut reloc_rel: (usize, usize, usize) = (0, 0, 16);
                let mut reloc_rela: (usize, usize, usize) = (0, 0, 24);
                for dent in dynamic_entries.iter() {
                    match dent.get_tag().map_err(|_| {
                        error! {"[LKM] invalid dynamic entry!"};
                        ()
                    })? {
                        Tag::JmpRel => {
                            reloc_jmprel.0 = dent.get_ptr().unwrap() as usize;
                        }
                        Tag::PltRelSize => {
                            reloc_jmprel.1 = dent.get_val().unwrap() as usize;
                        }
                        Tag::PltRel => {
                            reloc_jmprel.2 = if (dent.get_val().unwrap()) == 7 {
                                24
                            } else {
                                16
                            }
                        }
                        Tag::Rel => {
                            reloc_rel.0 = dent.get_ptr().unwrap() as usize;
                        }
                        Tag::RelSize => {
                            reloc_rel.1 = dent.get_val().unwrap() as usize;
                        }
                        Tag::Rela => {
                            reloc_rela.0 = dent.get_ptr().unwrap() as usize;
                        }
                        Tag::RelaSize => {
                            reloc_rela.1 = dent.get_val().unwrap() as usize;
                        }
                        _ => {}
                    }
                }
                // info!("[LKM] relocating three sections");
                self.reloc_symbols(mm, &elf, reloc_jmprel, base.0, dynsym_table, 0);
                self.reloc_symbols(mm, &elf, reloc_rel, base.0, dynsym_table, 0);
                self.reloc_symbols(mm, &elf, reloc_rela, base.0, dynsym_table, 0);
                // info!("[LKM] relocation done. adding module to manager and call init_module");
                let entry = get_symbol_addr(&elf, "entry");
                mm.entry = base + entry;
            }
        }
        Ok(())
    }

}

unsafe fn write_to_addr(mm:&mut MM, base: usize, offset: usize, val: usize) {
    let va = VirtAddr(base + offset);
    let pa = mm.translate(va).unwrap().0;
    *(pa as *mut usize) = val;
}
