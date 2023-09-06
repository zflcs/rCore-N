
#[cfg(feature = "board_axu15eg")]
pub mod axi_eth;
#[cfg(feature = "board_axu15eg")]
pub use axi_eth::*;


pub fn init() {
    axi_eth::init();
}
