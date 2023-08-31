

pub mod axi_eth;
pub use axi_eth::*;


pub fn init() {
    axi_eth::init();
}
