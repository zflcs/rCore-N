use crate::{mm, put_str, main};
use alloc::string::ToString;
use alloc::{collections::BTreeMap, string::String};
use xmas_elf::dynamic::Dynamic;
use super::structs::ModuleSymbol;

///
pub fn kernel_rt() -> BTreeMap<String, ModuleSymbol> {
    extern "C" {
        fn executor_ptr();
    }
    let mut symbols = BTreeMap::new();
    symbols.insert("put_str".to_string(), ModuleSymbol::create_symbol("put_char", put_str as _));
    symbols.insert("alloc".to_string(), ModuleSymbol::create_symbol("alloc", mm::alloc as _));
    symbols.insert("dealloc".to_string(), ModuleSymbol::create_symbol("dealloc", mm::dealloc as _));
    symbols.insert("main".to_string(), ModuleSymbol::create_symbol("main", main as _));
    symbols.insert("executor_ptr".to_string(), ModuleSymbol::create_symbol("executor_ptr", unsafe {executor_ptr as _}));

    symbols
}

///
pub fn user_rt<'a>(elf: &ElfFile<'a>) -> BTreeMap<String, ModuleSymbol> {
    let put_str_sym = get_symbol_addr(elf, "put_str");
    let alloc_sym = get_symbol_addr(elf, "alloc");
    let dealloc_sym = get_symbol_addr(elf, "dealloc");
    let main_sym = get_symbol_addr(elf, "main");
    let executor_ptr_sym = get_symbol_addr(elf, "executor_ptr");
    let mut symbols = BTreeMap::new();
    symbols.insert("put_str".to_string(), ModuleSymbol::create_symbol("put_str", put_str_sym));
    symbols.insert("alloc".to_string(), ModuleSymbol::create_symbol("alloc", alloc_sym));
    symbols.insert("dealloc".to_string(), ModuleSymbol::create_symbol("dealloc", dealloc_sym));
    symbols.insert("main".to_string(), ModuleSymbol::create_symbol("main", main_sym));
    symbols.insert("executor_ptr".to_string(), ModuleSymbol::create_symbol("executor_ptr", executor_ptr_sym));

    symbols
}


use alloc::vec::Vec;
use xmas_elf::sections::SectionData::{DynSymbolTable64, SymbolTable64, Dynamic64};
use xmas_elf::symbol_table::{Entry, Entry64, DynEntry64};
use xmas_elf::ElfFile;
type P64 = u64;


pub fn symbol_table<'a>(elf: &ElfFile<'a>) -> &'a [Entry64] {
    match elf.find_section_by_name(".symtab").unwrap().get_data(&elf).unwrap()
    {
        SymbolTable64(sym) => sym,
        _ => panic!("corrupted .symtab"),
    }
}

pub fn get_symbol_addr<'a>(elf: &ElfFile<'a>, symbol_name: &str) -> usize{
    let mut entry = 0 as usize;
    for sym  in symbol_table(elf){
        let name = sym.get_name(elf);
        if name.unwrap() == symbol_name{
            entry = sym.value() as usize;
        }
    }
    entry
}

pub fn dynsym_table<'a>(elf: &ElfFile<'a>) -> &'a [DynEntry64] {
    match elf.find_section_by_name(".dynsym").unwrap().get_data(&elf).unwrap()
    {
        DynSymbolTable64(dsym) => dsym,
        _ => panic!("corrupted .dynsym"),
    }
}

pub fn dynamic_table<'a>(elf: &ElfFile<'a>) -> &'a [Dynamic<P64>] {
    match elf.find_section_by_name(".dynamic").unwrap().get_data(&elf).unwrap()
    {
        Dynamic64(dsym) => dsym,
        _ => panic!("corrupted .dynamic"),
    }
}


pub fn dependency_table<'a>(elf: &ElfFile<'a>) -> Vec<(&'a str, usize)> {
    let mut res_vec = Vec::new();
    for sym  in symbol_table(elf){
        let name = sym.get_name(elf);
        if name.unwrap().contains("stub") {
            res_vec.push((sym.get_name(&elf).unwrap().trim_end_matches("_stub"), sym.value() as usize));
        }
    }
    res_vec
}
