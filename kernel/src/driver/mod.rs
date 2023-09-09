pub mod plic;
pub mod net;
pub mod dma;


pub fn init() {
    plic::init();
    dma::init();
    net::init();
    
}