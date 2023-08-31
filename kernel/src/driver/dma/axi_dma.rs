use axi_ethernet::XAE_MAX_FRAME_SIZE;
use lazy_static::lazy_static;
use alloc::{sync::Arc, boxed::Box};
use spin::Mutex;
use axidma::{AxiDma, AxiDmaIntr, AXI_DMA_CONFIG, RX_FRAMES, TX_FRAMES};
use core::sync::atomic::Ordering::Relaxed;

lazy_static! {
    pub static ref AXI_DMA: Arc<Mutex<AxiDma>> = Arc::new(Mutex::new(AxiDma::default()));
    pub static ref AXI_DMA_INTR: Arc<Mutex<AxiDmaIntr>> =
        Arc::new(Mutex::new(AxiDmaIntr::new(AXI_DMA_CONFIG.base_address)));
}

const RX_BD_CNT: usize = 1024;
const TX_BD_CNT: usize = 1024;


pub fn init() {
    // 初始化发送帧计数和接收帧计数
    TX_FRAMES.store(0, Relaxed);
    RX_FRAMES.store(0, Relaxed);
    // 初始化 BD
    AXI_DMA.lock().tx_bd_create(TX_BD_CNT);
    AXI_DMA.lock().rx_bd_create(RX_BD_CNT);
    // 中断使能
    AXI_DMA.lock().tx_intr_enable();
    AXI_DMA.lock().rx_intr_enable();
    // 提交接收的缓冲区
    let mut rx_frame = Box::pin([0u8; XAE_MAX_FRAME_SIZE]);
    AXI_DMA.lock().rx_submit(&[&rx_frame]);
    AXI_DMA.lock().rx_to_hw();

}
