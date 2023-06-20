pub mod plic;
pub mod uart;
mod bus;
mod net;

#[cfg(feature = "board_qemu")]
pub use net::NetDevice;

pub fn init() {
    net::init();
}
