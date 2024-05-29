mod bus;
mod net;
pub mod plic;
pub mod uart;

pub use net::NetDevice;

pub fn init() {
    net::init();
}
