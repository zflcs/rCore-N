OUTPUT_ARCH(riscv)
ENTRY(__libc_start_main)

BASE_ADDRESS = 0x1000;

SECTIONS
{
    . = BASE_ADDRESS;
    .text : {
        *(.text.entry)
        *(.text .text.*)
    }
    . = ALIGN(4K);
    .rodata : {
        *(.rodata .rodata.*)
        *(.srodata .srodata.*)
    }
    . = ALIGN(4K);
    .data : {
        *(.data.heap)
        executor_ptr = .;
        *(.data.executor)
        *(.data .data.*)
        *(.sdata .sdata.*)
    }
    .bss : {
        *(.bss .bss.*)
        *(.sbss .sbss.*)
        *(.bss .bss.memory)
    }

    /DISCARD/ : {
        *(.eh_frame)
        *(.debug*)
    }
}