#[cfg(feature = "board_axu15eg")]
pub mod axu15eg;
#[cfg(feature = "board_axu15eg")]
pub use axu15eg::*;

#[cfg(feature = "board_qemu")]
pub mod virtio;
#[cfg(feature = "board_qemu")]
pub use virtio::*;


