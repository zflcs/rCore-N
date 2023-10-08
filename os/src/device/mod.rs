pub mod plic;
pub mod uart;
mod bus;
pub mod net;

pub use net::NET_DEVICE;

pub fn init() {
    net::init();
}
