use spin::{Lazy, Mutex};

use crate::config::MEMORY_END;
use super::*;

extern "C" {
    fn stext();
    fn etext();
    fn srodata();
    fn erodata();
    fn sdata();
    fn edata();
    fn sbss_with_stack();
    fn ebss();
    fn ekernel();
    fn strampoline();
}

pub static KERNEL_SPACE: Lazy<Arc<Mutex<MM>>> = Lazy::new(|| Arc::new(Mutex::new(new_kernel().unwrap())));

pub fn kernel_token() -> usize {
    KERNEL_SPACE.lock().token()
}

///
pub fn kernel_activate() {
    let satp = kernel_token();
    unsafe {
        riscv::register::satp::write(satp);
        riscv::asm::sfence_vma_all();
    }
}

/// Without kernel stacks.
fn new_kernel() -> Result<MM> {
    let mut mm = MM::new()?;


    // Map kernel .text section
    mm.alloc_write_vma(
        None,
        (stext as usize).into(),
        (etext as usize).into(),
        VMFlags::READ | VMFlags::EXEC | VMFlags::IDENTICAL,
    )?;
    info!(
        "{:>10} [{:#x}, {:#x})",
        ".text", stext as usize, etext as usize
    );
    // Map kernel .rodata section
    mm.alloc_write_vma(
        None,
        (srodata as usize).into(),
        (erodata as usize).into(),
        VMFlags::READ | VMFlags::IDENTICAL,
    )?;
    info!(
        "{:>10} [{:#x}, {:#x})",
        ".rodata", srodata as usize, erodata as usize
    );

    // Map kernel .data section
    mm.alloc_write_vma(
        None,
        (sdata as usize).into(),
        (edata as usize).into(),
        VMFlags::READ | VMFlags::WRITE | VMFlags::IDENTICAL,
    )?;
    info!(
        "{:>10} [{:#x}, {:#x})",
        ".data", sdata as usize, edata as usize
    );

    // Map kernel .bss section
    mm.alloc_write_vma(
        None,
        (sbss_with_stack as usize).into(),
        (ebss as usize).into(),
        VMFlags::READ | VMFlags::WRITE | VMFlags::IDENTICAL,
    )?;
    info!(
        "{:>10} [{:#x}, {:#x})",
        ".bss", sbss_with_stack as usize, ebss as usize
    );

    // Physical memory area
    mm.alloc_write_vma(
        None,
        (ekernel as usize).into(),
        MEMORY_END.into(),
        VMFlags::READ | VMFlags::WRITE | VMFlags::IDENTICAL,
    )?;
    info!(
        "{:>10} [{:#x}, {:#x})",
        "mem", ekernel as usize, MEMORY_END
    );

    // plic
    mm.alloc_write_vma(
        None,
        0xc00_0000.into(),
        0x1000_0000.into(),
        VMFlags::READ | VMFlags::WRITE | VMFlags::IDENTICAL,
    )?;
    info!(
        "{:>10} [{:#x}, {:#x})",
        "plic", 0xc00_0000, 0x1000_0000
    );

    #[cfg(feature = "board_qemu")]
    {
        mm.alloc_write_vma(
            None,
            0x1000_6000.into(),
            0x1000_9000.into(),
            VMFlags::READ | VMFlags::WRITE | VMFlags::IDENTICAL,
        )?;
        info!(
            "{:>10} [{:#x}, {:#x})",
            "mmio", 0x1000_6000, 0x1000_9000
        );
    }
    #[cfg(feature = "board_axu15eg")]
    {
        mm.alloc_write_vma(
            None,
            0x6000_0000.into(),
            0x6200_0000.into(),
            VMFlags::READ | VMFlags::WRITE | VMFlags::IDENTICAL,
        )?;
        info!(
            "{:>10} [{:#x}, {:#x})",
            "mmio", 0x6000_0000, 0x6200_0000
        );
    }
    mm.start_brk = MEMORY_END.into();
    unsafe { core::arch::asm!("fence.i") }
    log::debug!("{:?}", mm);
    Ok(mm)
}