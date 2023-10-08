use lazy_static::lazy_static;
use alloc::sync::Arc;
use axidma::{AxiDma, AxiDmaIntr, AXI_DMA_CONFIG, RX_FRAMES, TX_FRAMES};
use core::{sync::atomic::Ordering::Relaxed, pin::Pin};
use kernel_sync::SpinLock;

#[repr(C, align(64))]
pub struct Buf([u8; 1024]);

static mut TX_BUF: Buf = Buf([0u8; 1024]);
static mut RX_BUF: Buf = Buf([0u8; 1024]);

lazy_static! {
    pub static ref AXI_DMA: Arc<SpinLock<AxiDma>> = Arc::new(SpinLock::new(AxiDma::new(AXI_DMA_CONFIG, Pin::new(unsafe {&mut TX_BUF.0}), Pin::new(unsafe {&mut RX_BUF.0}))));
    pub static ref AXI_DMA_INTR: Arc<SpinLock<AxiDmaIntr>> =
        Arc::new(SpinLock::new(AxiDmaIntr::new(AXI_DMA_CONFIG.base_address)));
}

const RX_BD_CNT: usize = 2048;
const TX_BD_CNT: usize = 2048;


pub fn init() {
    let mut axi_dma = AXI_DMA.lock();
    axi_dma.reset();
    axi_dma.tx_cyclic_enable();
    axi_dma.rx_cyclic_enable();
    // 初始化 BD
    axi_dma.tx_bd_create(TX_BD_CNT);
    axi_dma.rx_bd_create(RX_BD_CNT);
    // 中断使能
    axi_dma.tx_intr_enable();
    axi_dma.rx_intr_enable();
    // 提交接收的缓冲区
    // let rx_frame = Box::pin([0u8; XAE_MAX_FRAME_SIZE]);
    axi_dma.rx_submit();
    axi_dma.rx_to_hw();    
}
