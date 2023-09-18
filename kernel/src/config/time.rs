/// Clock frequency (platform dependent).
#[cfg(feature = "board_qemu")]
pub const CLOCK_FREQ: usize = 1250_0000;

#[cfg(feature = "board_axu15eg")]
pub const CLOCK_FREQ: usize = 1000_0000;
