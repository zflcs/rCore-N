#[cfg(feature = "board_qemu")]
mod virtio_net;
#[cfg(feature = "board_qemu")]
pub use virtio_net::*;

#[cfg(feature = "board_axu15eg")]
mod axi_eth;
#[cfg(feature = "board_axu15eg")]
pub use axi_eth::*;

pub fn init() {
    #[cfg(feature = "board_qemu")]
    virtio_net::init();
    #[cfg(feature = "board_axu15eg")]
    axi_eth::init();
}
