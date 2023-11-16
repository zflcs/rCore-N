
.altmacro
.macro SAVE_SN n
    sd s\n, (\n+1)*8(a0)
.endm
.macro LOAD_SN n
    ld s\n, (\n+1)*8(a1)
.endm
    .section .text
    .globl __switch
__switch:
    # __switch(
    #     current_task_cx_ptr: &*const TaskContext,
    #     next_task_cx_ptr: &*const TaskContext
    # )
    # push TaskContext to current sp and save its address to where a0 points to
    # fill TaskContext with ra & s0-s11
    sd sp, 14*8(a0)
    sd ra, 0(a0)
    .set n, 0
    .rept 12
        SAVE_SN %n
        .set n, n + 1
    .endr

    # ready for loading TaskContext a1 points to
    # load registers in the TaskContext
    ld sp, 14*8(a1)
    ld ra, 0(a1)
    .set n, 0
    .rept 12
        LOAD_SN %n
        .set n, n + 1
    .endr
    ret

