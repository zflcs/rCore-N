use core::slice;

use alloc::sync::Arc;
use spin::{Mutex, Lazy};
use easy_fs::BlockDevice;



pub static BLOCK_DEVICE: Lazy<Arc<dyn BlockDevice>> = Lazy::new(|| Arc::new(RamFS::new()));

pub struct RamFS(usize);

impl RamFS {
    pub fn new() -> Self {
        extern "C" {
            fn sramfs();
        }
        Self(sramfs as _)
    }
}

const BLOCK_SZ: usize = 512;

impl BlockDevice for RamFS {
    fn read_block(&self, block_id: usize, buf: &mut [u8]) {
        let start_ptr = self.0;
        let target_ptr = start_ptr + block_id * BLOCK_SZ;
        let target_slice = unsafe { slice::from_raw_parts(target_ptr as *const u8, BLOCK_SZ) };
        buf.copy_from_slice(&target_slice[0..BLOCK_SZ]);
    }

    fn write_block(&self, block_id: usize, buf: &[u8]) {
        let start_ptr = self.0;
        let target_ptr = start_ptr + block_id * BLOCK_SZ;
        let target_slice = unsafe { slice::from_raw_parts_mut(target_ptr as *mut u8, BLOCK_SZ) };
        target_slice.copy_from_slice(&buf)
    }

    fn handle_irq(&self) {
        unimplemented!();
    }
}