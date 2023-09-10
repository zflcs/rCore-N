/// Tampoline page stores naked code (mostly ASM) to provide the bridge between user and
/// kernel space. The page is mapped at the same virtual address in user and kernel space.
/// **RISC-V** hardware doesn't switch page tables during a trap, we need the user page table
/// to include a mapping for the trap vector instructions that `stvec` points to.
/// In this way, after switching page table root in `satp` register, virtual memory is still
/// the same, so it can continue to execute without crash.
#[naked]
#[no_mangle]
#[allow(named_asm_labels)]
#[link_section = ".text.trampoline"]
pub unsafe extern "C" fn __trampoline() {
    core::arch::asm!(
        // Jump to stvec (user trap entry)
        "
        .align 4
        .globl __uservec
    __uservec:
        ",
        // Now sp points to user trapframe, and sscratch points to user stack.
        "csrrw sp, sscratch, sp",
        // Save user registers in trapframe.
        "
        sd ra, 40(sp)
        sd gp, 56(sp)
        sd tp, 64(sp)
        sd t0, 72(sp)
        sd t1, 80(sp)
        sd t2, 88(sp)
        sd s0, 96(sp)
        sd s1, 104(sp)
        sd a0, 112(sp)
        sd a1, 120(sp)
        sd a2, 128(sp)
        sd a3, 136(sp)
        sd a4, 144(sp)
        sd a5, 152(sp)
        sd a6, 160(sp)
        sd a7, 168(sp)
        sd s2, 176(sp)
        sd s3, 184(sp)
        sd s4, 192(sp)
        sd s5, 200(sp)
        sd s6, 208(sp)
        sd s7, 216(sp)
        sd s8, 224(sp)
        sd s9, 232(sp)
        sd s10, 240(sp)
        sd s11, 248(sp)
        sd t3, 256(sp)
        sd t4, 264(sp)
        sd t5, 272(sp)
        sd t6, 280(sp)
        csrr t0, sscratch
        sd t0, 48(sp)
        ",
        // Save sepc and sstatus
        "
        csrr t0, sepc
        csrr t1, sstatus
        sd t0, 24(sp)
        sd t1, 32(sp)
        ",
        // Load the virtual address of trap handler
        "ld t0, 16(sp)",
        // Load the kernel page table root address
        "ld t1, 0(sp)",
        // Load cpu id
        "ld tp, 288(sp)",
        // Initialize kernel stack pointer
        "ld sp, 8(sp)",
        // Change to the kernel page table root
        "csrw satp, t1",
        // Flush all satle TLB entries
        "sfence.vma zero, zero",
        // Jump to trap handler
        "jr t0",
        // __userret(trapframe_va, page_table_root)
        // switch from kernel to user
        "
        .globl __userret
    __userret:
        ",
        // Restore user page table (see __uservec)
        "
        csrw satp, a1
        sfence.vma zero, zero
        ",
        // Now sscratch agiain points to user trapframe
        "csrw sscratch, a0",
        // Save cpu id
        "sd tp, 288(a0)",
        // Restore sepc and sstatus
        "
        ld t0, 24(a0)
        ld t1, 32(a0)
        csrw sepc, t0
        csrw sstatus, t1
        ",
        // Restore user registers
        "
        ld ra, 40(a0)
        ld sp, 48(a0)
        ld gp, 56(a0)
        ld tp, 64(a0)
        ld t0, 72(a0)
        ld t1, 80(a0)
        ld t2, 88(a0)
        ld s0, 96(a0)
        ld s1, 104(a0)
        ld a1, 120(a0)
        ld a2, 128(a0)
        ld a3, 136(a0)
        ld a4, 144(a0)
        ld a5, 152(a0)
        ld a6, 160(a0)
        ld a7, 168(a0)
        ld s2, 176(a0)
        ld s3, 184(a0)
        ld s4, 192(a0)
        ld s5, 200(a0)
        ld s6, 208(a0)
        ld s7, 216(a0)
        ld s8, 224(a0)
        ld s9, 232(a0)
        ld s10, 240(a0)
        ld s11, 248(a0)
        ld t3, 256(a0)
        ld t4, 264(a0)
        ld t5, 272(a0)
        ld t6, 280(a0)
        ",
        
        // Finally restore a0
        "ld a0, 112(a0)",
        // Return to user context
        "sret",
        // kernel trap vector
        "
        .globl __kernelvec
    __kernelvec:
        ",
        // Allocate stack space
        "addi sp, sp, -248",
        // Save kernel registers
        "
        sd ra, 0(sp)
        sd gp, 8(sp)
        ",
        // Skip tp, used for cpu id
        "
        sd t0, 16(sp)
        sd t1, 24(sp)
        sd t2, 32(sp)
        sd s0, 40(sp)
        sd s1, 48(sp)
        sd a0, 56(sp)
        sd a1, 64(sp)
        sd a2, 72(sp)
        sd a3, 80(sp)
        sd a4, 88(sp)
        sd a5, 96(sp)
        sd a6, 104(sp)
        sd a7, 112(sp)
        sd s2, 120(sp)
        sd s3, 128(sp)
        sd s4, 136(sp)
        sd s5, 144(sp)
        sd s6, 152(sp)
        sd s7, 160(sp)
        sd s8, 168(sp)
        sd s9, 176(sp)
        sd s10, 184(sp)
        sd s11, 192(sp)
        sd t3, 200(sp)
        sd t4, 208(sp)
        sd t5, 216(sp)
        sd t6, 224(sp)
        ",
        // Save sepc and sstatus
        "
        csrr t0, sepc
        csrr t1, sstatus
        sd t0, 232(sp)
        sd t1, 240(sp)
        ",
        // Return new sp (*KernelTrapContext)
        "mv a0, sp",
        // Jump to kernel_trap_handler(&KernelTrapContext)
        "
        csrr t2, sscratch
        jalr t2
        ",
        // kernel return
        "
        .globl __kernelret
    __kernelret:
        ",
        // restore sstatus and sepc
        "
        ld t1, 240(sp)
        ld t0, 232(sp)
        csrw sstatus, t1
        csrw sepc, t0
        ",
        // restore general register
        "
        ld t6, 224(sp)
        ld t5, 216(sp)
        ld t4, 208(sp)
        ld t3, 200(sp)
        ld s11, 192(sp)
        ld s10, 184(sp)
        ld s9, 176(sp)
        ld s8, 168(sp)
        ld s7, 160(sp)
        ld s6, 152(sp)
        ld s5, 144(sp)
        ld s4, 136(sp)
        ld s3, 128(sp)
        ld s2, 120(sp)
        ld a7, 112(sp)
        ld a6, 104(sp)
        ld a5, 96(sp)
        ld a4, 88(sp)
        ld a3, 80(sp)
        ld a2, 72(sp)
        ld a1, 64(sp)
        ld a0, 56(sp)
        ld s1, 48(sp)
        ld s0, 40(sp)
        ld t2, 32(sp)
        ld t1, 24(sp)
        ld t0, 16(sp)
        ",
        // restore ra gp
        "
        ld gp, 8(sp) 
        ld ra, 0(sp)
        ",
        // restore sp
        "addi sp, sp, 248",
        "sret",
        options(noreturn),
    );
}
