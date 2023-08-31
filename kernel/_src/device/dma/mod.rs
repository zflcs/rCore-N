

#[cfg(feature = "board_axu15eg")]
mod axi_dma;

#[cfg(feature = "board_axu15eg")]
pub use axi_dma::*;

// #[cfg(feature = "board_axu15eg")]
// pub use axi_dma::AXI_DMA as DMA;

pub fn init() {
    #[cfg(feature = "board_axu15eg")]
    axi_dma::init();
}