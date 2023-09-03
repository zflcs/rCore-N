


use alloc::{sync::Arc, boxed::Box, vec::Vec};
use axi_ethernet::*;
use lazy_static::lazy_static;
use spin::Mutex;
use log::trace;

use crate::driver::dma::AXI_DMA;

pub const AXI_ETHERNET_BASE_ADDR: usize = 0x60140000;
pub const MAC_ADDR: [u8; 6] = [0x00, 0x0A, 0x35, 0x01, 0x02, 0x03];

pub struct NetDevice;
use axidma::{AXI_DMA_CONFIG, TX_FRAMES, RX_FRAMES};
use core::sync::atomic::Ordering::Relaxed;

impl NetDevice {

    // data 中需要包含源地址、目的地址、eth type/len 信息
    fn fill_frame(&self, tx_frame: &mut [u8], data: &[u8]) {
        trace!("fill tx frame");
        // fill payload
        let payload_size = data.len();
        tx_frame[0..0 + payload_size].copy_from_slice(data);
        trace!("fill tx frame success");
    }

    pub fn transmit(&self, data: &[u8]) {
        trace!("net transmit");
        // 初始化填充发送帧
        let prev_tx_cnt = TX_FRAMES.load(Relaxed);
        let mut tx_frame = Box::pin([0u8; XAE_MAX_FRAME_SIZE]);
        self.fill_frame(tx_frame.as_mut_slice(), data);
        AXI_DMA.lock().tx_submit(&[&tx_frame]);
        AXI_DMA.lock().tx_to_hw();
        // 等待 dma 产生 mm2s 中断
        while TX_FRAMES.load(Relaxed) == prev_tx_cnt { }
    }

    pub fn receive(&self) -> Option<Vec<u8>> {
        // 将数据复制到 buffer 中，这里只从 buf0 读取数据，暂时没有用到 dma 的更多功能
        if let Some(bufs) = AXI_DMA.lock().rx_from_hw() {
            Some(bufs[0].to_vec())
        } else {
            None
        }
    }

    pub fn recycle_rx_buffer(&self, buf: Vec<u8>) {
        drop(buf)
    }
    
}



lazy_static! {
    pub static ref ETHERNET: Arc<Mutex<AxiEthernet>> = Arc::new(Mutex::new(AxiEthernet::new(AXI_ETHERNET_BASE_ADDR, AXI_DMA_CONFIG.base_address)));
}

pub fn init() {
    let mut speed = 1000;
    ETHERNET.lock().reset();
    ETHERNET.lock().detect_phy();
    speed = ETHERNET.lock().get_phy_speed_ksz9031();
    trace!("speed is: {}", speed);
    ETHERNET.lock().set_operating_speed(speed as u16);
    if speed == 0 {
        ETHERNET.lock().link_status = LinkStatus::EthLinkDown;
    } else {
        ETHERNET.lock().link_status = LinkStatus::EthLinkUp;
    }
    trace!("link_status: {:?}", ETHERNET.lock().link_status);
    for _ in 0..100000 {}
    for _ in 0..100000 {} 
    ETHERNET.lock().enable_intr(XAE_INT_RECV_ERROR_MASK);
}





