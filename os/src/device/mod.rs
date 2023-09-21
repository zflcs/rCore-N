pub mod plic;
pub mod uart;
mod bus;
pub mod net;
pub mod dma;

pub use net::NetDevice;
pub use dma::*;

pub fn init() {
    net::init();
    dma::init();
}
