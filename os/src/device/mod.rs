mod net;
pub mod plic;
#[cfg(feature = "board_qemu")]
mod virtio_bus;

pub use net::NET_DEVICE;

pub fn init() {
    net::init();
}
