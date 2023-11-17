use spin::Mutex;
use alloc::string::*;
use alloc::sync::Arc;
use alloc::vec::*;

use crate::mm::VirtAddr;

#[derive(PartialEq)]
pub struct ModuleSymbol {
    pub name: String,
    pub loc: usize,
}

impl ModuleSymbol {
    pub fn create_symbol(symbol_name: &str, symbol_loc: usize) -> Self {
        Self {
            name: String::from(symbol_name),
            loc: symbol_loc,
        }
    }
}

pub enum ModuleState {
    Ready,
    PrepareUnload,
    Unloading,
}

pub struct ModuleRef;
pub struct LoadedModule {
    pub name: String,
    pub exported_symbols: Vec<ModuleSymbol>,
    pub used_counts: i32,
    pub using_counts: Arc<ModuleRef>,
    pub vspace: (usize, usize),
    pub lock: Mutex<()>,
    pub state: ModuleState,
}

impl LoadedModule {
    // Grabs a reference to the kernel module.
    // For example, a file descriptor to a device file controlled by the module is a reference.
    // This must be called without the lock!
    pub fn grab(&self) -> Arc<ModuleRef> {
        Arc::clone(&self.using_counts)
    }
}


