pub mod plic;
#[cfg(feature = "board_qemu")]
mod virtio_bus;
mod net;

pub use net::NET_DEVICE;


pub fn init() {
    net::init();
}
