use crate::{mm, put_char, main};
use alloc::string::ToString;
use alloc::{collections::BTreeMap, string::String};
use super::structs::ModuleSymbol;

///
pub fn kernel_rt() -> BTreeMap<String, ModuleSymbol> {
    extern "C" {
        fn executor_ptr();
    }
    let mut symbols = BTreeMap::new();
    symbols.insert("put_char".to_string(), ModuleSymbol::create_symbol("put_char", put_char as _));
    symbols.insert("alloc".to_string(), ModuleSymbol::create_symbol("alloc", mm::alloc as _));
    symbols.insert("dealloc".to_string(), ModuleSymbol::create_symbol("dealloc", mm::dealloc as _));
    symbols.insert("main".to_string(), ModuleSymbol::create_symbol("main", main as _));
    symbols.insert("executor_ptr".to_string(), ModuleSymbol::create_symbol("executor_ptr", unsafe {executor_ptr as _}));

    symbols
}

///
pub fn from_elf<'a>(elf: &ElfFile<'a>) -> BTreeMap<String, ModuleSymbol> {
    let put_char_sym = get_symbol_addr(elf, "put_char");
    let alloc_sym = get_symbol_addr(elf, "alloc");
    let dealloc_sym = get_symbol_addr(elf, "dealloc");
    let main_sym = get_symbol_addr(elf, "main");
    let executor_ptr_sym = get_symbol_addr(elf, "executor_ptr");
    let mut symbols = BTreeMap::new();
    symbols.insert("put_char".to_string(), ModuleSymbol::create_symbol("put_char", put_char_sym));
    symbols.insert("alloc".to_string(), ModuleSymbol::create_symbol("alloc", alloc_sym));
    symbols.insert("dealloc".to_string(), ModuleSymbol::create_symbol("dealloc", dealloc_sym));
    symbols.insert("main".to_string(), ModuleSymbol::create_symbol("main", main_sym));
    symbols.insert("executor_ptr".to_string(), ModuleSymbol::create_symbol("executor_ptr", executor_ptr_sym));

    symbols
}


use alloc::vec::Vec;
use xmas_elf::sections::SectionData::SymbolTable64;
use xmas_elf::symbol_table::{Entry, Entry64};
use xmas_elf::ElfFile;
type P64 = u64;


fn symbol_table<'a>(elf: &ElfFile<'a>) -> &'a [Entry64] {
    match elf.find_section_by_name(".symtab").unwrap().get_data(&elf).unwrap()
    {
        SymbolTable64(dsym) => dsym,
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

