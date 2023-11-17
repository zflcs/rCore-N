/// user stack size
pub const USER_STACK_SIZE: usize = 0x4000;
/// kernel stack size
pub const KERNEL_STACK_SIZE: usize = 0x4000;
/// kernel heap size
pub const KERNEL_HEAP_SIZE: usize = 0x80_0000;
/// the amount of cpu
pub const CPU_NUM: usize = 4;
/// the physical memory end
pub const MEMORY_END: usize = 0x84000000;
/// page size: 4K
pub const PAGE_SIZE: usize = 0x1000;
///
pub const PAGE_SIZE_BITS: usize = 0xc;
/// the base address of trampoline
pub const TRAMPOLINE: usize = usize::MAX - PAGE_SIZE + 1;
/// the address of trap record
pub const USER_TRAP_BUFFER: usize = TRAMPOLINE - PAGE_SIZE;
/// the address of heap pointer
pub const HEAP_BUFFER: usize = USER_TRAP_BUFFER - PAGE_SIZE;
/// the trap context of user thread 0
pub const TRAP_CONTEXT: usize = HEAP_BUFFER - PAGE_SIZE;

/// The highest virtual address of the low 256 GB in SV39.
// pub const LOW_MAX_VA: usize = 0x0000_003F_FFFF_FFFF;
pub const LOW_MAX_VA: usize = 0xFFFF_FFFF;
/// User heap size
pub const USER_HEAP_SIZE: usize = 0x20_0000;
/// User maximum pages
pub const USER_MAX_PAGES: usize = (LOW_MAX_VA + 1) >> PAGE_SIZE_BITS;

/// Maximum virtual memory areas in an address space
pub const MAX_MAP_COUNT: usize = 256;


#[cfg(feature = "board_qemu")]
/// the clock frequency in qemu
pub const CLOCK_FREQ: usize = 12500000;

#[cfg(feature = "board_axu15eg")]
/// the clock frequency in axu15eg
pub const CLOCK_FREQ: usize = 10_000_000;

#[cfg(feature = "board_axu15eg")]
pub use axi_net::*;

#[cfg(feature = "board_axu15eg")]
mod axi_net {
    use axi_dma::AxiDmaConfig;
    use axi_ethernet::{XAE_MAX_FRAME_SIZE, XAE_MAX_JUMBO_FRAME_SIZE};

    pub const AXI_DMA_CONFIG: AxiDmaConfig = AxiDmaConfig {
        device_id: 0,
        base_address: 0x6010_0000,
        has_sts_cntrl_strm: false,
        is_micro_dma: false,
        has_mm2s: true,
        has_mm2s_dre: false,
        mm2s_data_width: 64,
        mm2s_burst_size: 16,
        has_s2mm: true,
        has_s2mm_dre: false,
        s2mm_data_width: 64,
        s2mm_burst_size: 16,
        has_sg: true,
        sg_length_width: 16,
        addr_width: 64,
    };

    pub struct AxiNetConfig {
        pub tx_bd_cnt: usize,
        pub rx_bd_cnt: usize,
        pub eth_baseaddr: usize,
        pub dma_baseaddr: usize,
        pub mac_addr: [u8; 6],
        pub mtu: usize,
    }

    pub const AXI_NET_CONFIG: AxiNetConfig = AxiNetConfig {
        tx_bd_cnt: 1024,
        rx_bd_cnt: 1024,
        eth_baseaddr: 0x60140000,
        dma_baseaddr: 0x6010_0000,
        mac_addr: [0x00, 0x0A, 0x35, 0x01, 0x02, 0x03],
        mtu: XAE_MAX_JUMBO_FRAME_SIZE,
    };
}
