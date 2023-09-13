
#[cfg(feature = "board_axu15eg")]
pub mod axi_eth;
#[cfg(feature = "board_axu15eg")]
pub use axi_eth::*;


pub fn init() {
    #[cfg(feature = "board_axu15eg")]
    axi_eth::init();
}
