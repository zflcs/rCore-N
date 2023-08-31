pub mod plic;
pub mod uart;
mod bus;
mod net;
mod dma;
mod uintc;

pub use net::NetDevice;
pub use dma::*;

pub fn init() {
    net::init();
    dma::init();
    uintc::init();
}
