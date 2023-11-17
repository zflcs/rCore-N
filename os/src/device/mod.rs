mod net;
// pub mod plic;
pub mod ramfs;
pub use ramfs::BLOCK_DEVICE;

#[cfg(feature = "board_qemu")]
mod virtio_bus;

pub use net::NET_DEVICE;

pub fn init() {
    net::init();
}
