///RISC-V Relocation Constants.
pub const R_RISCV_NONE: usize = 0;
#[allow(unused)]
pub const R_RISCV_32: usize = 1;
pub const R_RISCV_64: usize = 2;
pub const R_RISCV_RELATIVE: usize = 3;
#[allow(unused)]
pub const R_RISCV_COPY: usize = 4;
pub const R_RISCV_JUMP_SLOT: usize = 5;
#[allow(unused)]
pub const R_RISCV_TLS_DTPMOD32: usize = 6;
#[allow(unused)]
pub const R_RISCV_TLS_DTPMOD64: usize = 7;
#[allow(unused)]
pub const R_RISCV_TLS_DTPREL32: usize = 8;
#[allow(unused)]
pub const R_RISCV_TLS_DTPREL64: usize = 9;
#[allow(unused)]
pub const R_RISCV_TLS_TPREL32: usize = 10;
#[allow(unused)]
pub const R_RISCV_TLS_TPREL64: usize = 11;

pub const REL_NONE: usize = R_RISCV_NONE;
#[cfg(target_arch = "riscv32")]
pub const REL_SYMBOLIC: usize = R_RISCV_32;
#[cfg(target_arch = "riscv64")]
pub const REL_SYMBOLIC: usize = R_RISCV_64;
pub const REL_OFFSET32: usize = 0; // dunno
pub const REL_GOT: usize = 0; // dunno
pub const REL_PLT: usize = R_RISCV_JUMP_SLOT;
pub const REL_RELATIVE: usize = R_RISCV_RELATIVE;
#[allow(unused)]
pub const REL_COPY: usize = R_RISCV_COPY;
#[cfg(target_arch = "riscv32")]
#[allow(unused)]
pub const REL_DTPMOD: usize = R_RISCV_TLS_DTPMOD32;
#[cfg(target_arch = "riscv64")]
#[allow(unused)]
pub const REL_DTPMOD: usize = R_RISCV_TLS_DTPMOD64;
#[cfg(target_arch = "riscv32")]
pub const REL_DTPOFF: usize = R_RISCV_TLS_DTPREL32;
#[cfg(target_arch = "riscv64")]
#[allow(unused)]
pub const REL_DTPOFF: usize = R_RISCV_TLS_DTPREL64;
#[cfg(target_arch = "riscv32")]
pub const REL_TPOFF: usize = R_RISCV_TLS_TPREL32;
#[cfg(target_arch = "riscv64")]
#[allow(unused)]
pub const REL_TPOFF: usize = R_RISCV_TLS_TPREL64;
#[allow(unused)]
pub const REL_TLSDESC: usize = 0; // dunno