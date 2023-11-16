
../target/riscv64gc-unknown-linux-gnu/debug/hello:     file format elf64-littleriscv

SYMBOL TABLE:
0000000000001000 l    d  .text	0000000000000000 .text
0000000000002000 l    d  .interp	0000000000000000 .interp
0000000000002028 l    d  .dynsym	0000000000000000 .dynsym
0000000000002058 l    d  .dynstr	0000000000000000 .dynstr
0000000000002060 l    d  .hash	0000000000000000 .hash
0000000000002078 l    d  .gnu.hash	0000000000000000 .gnu.hash
0000000000002094 l    d  .note.ABI-tag	0000000000000000 .note.ABI-tag
0000000000003000 l    d  .dynamic	0000000000000000 .dynamic
0000000000003120 l    d  .got	0000000000000000 .got
0000000000000000 l    d  .comment	0000000000000000 .comment
0000000000000000 l    d  .riscv.attributes	0000000000000000 .riscv.attributes
0000000000000000 l    df *ABS*	0000000000000000 hello.d83ba15731b93d11-cgu.0
0000000000000000 l    df *ABS*	0000000000000000 abi-note.c
0000000000002094 l     O .note.ABI-tag	0000000000000020 __abi_tag
0000000000000000 l    df *ABS*	0000000000000000 start.os
0000000000000000 l    df *ABS*	0000000000000000 init.c
0000000000003000 l     O *ABS*	0000000000000000 _DYNAMIC
0000000000001020 l     O *ABS*	0000000000000000 _PROCEDURE_LINKAGE_TABLE_
0000000000003120 l     O *ABS*	0000000000000000 _GLOBAL_OFFSET_TABLE_
0000000000001000 g       *ABS*	0000000000000000 BASE_ADDRESS
0000000000001012 g     F .text	0000000000000004 main
0000000000001000 g     F .text	0000000000000012 __libc_start_main



Disassembly of section .text:

0000000000001000 <__libc_start_main>:
    1000:	1141                	add	sp,sp,-16
    1002:	e406                	sd	ra,8(sp)
    1004:	00000097          	auipc	ra,0x0
    1008:	00e080e7          	jalr	14(ra) # 1012 <main>
    100c:	60a2                	ld	ra,8(sp)
    100e:	0141                	add	sp,sp,16
    1010:	8082                	ret

0000000000001012 <main>:
    1012:	4501                	li	a0,0
    1014:	8082                	ret
