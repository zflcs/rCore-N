


use alloc::{sync::Arc, boxed::Box, vec::Vec};
use axi_ethernet::*;
use lazy_static::lazy_static;
use kernel_sync::SpinLock;
use log::trace;

use crate::driver::dma::AXI_DMA;

pub const AXI_ETHERNET_BASE_ADDR: usize = 0x60140000;
pub const MAC_ADDR: [u8; 6] = [0x00, 0x0A, 0x35, 0x01, 0x02, 0x03];

pub struct NetDevice;
use axidma::{AXI_DMA_CONFIG, TX_FRAMES};
use core::sync::atomic::Ordering::Relaxed;
const PAYLOAD_SIZE: usize = 1024;

impl NetDevice {

    // data 中需要包含源地址、目的地址、eth type/len 信息
    fn fill_frame(&self, tx_frame: &mut [u8], data: &[u8]) {
        log::trace!("fill tx frame");
        // fill payload
        let payload_size: usize = data.len();
        tx_frame[0..0 + payload_size].copy_from_slice(data);
        log::trace!("fill tx frame success");
    }

    pub fn transmit(&self, data: &[u8]) {
        log::trace!("net transmit");
        // 初始化填充发送帧
        let mut buf = Vec::<u8>::new();
        buf.resize_with(data.len(), || 0);
        let mut tx_frame = Box::pin(buf);
        log::trace!("frame len {}", tx_frame.len());
        // let mut tx_frame = Box::pin([0u8; PAYLOAD_SIZE]);
        self.fill_frame(tx_frame.as_mut_slice(), data);
        AXI_DMA.lock().tx_submit(&[&tx_frame]);
        AXI_DMA.lock().tx_to_hw();
        
    }

    pub fn receive(&self) -> Option<Vec<u8>> {
        // 将数据复制到 buffer 中，这里只从 buf0 读取数据，暂时没有用到 dma 的更多功能
        let mut axi_dma = AXI_DMA.lock();
        if let Some(mut bufs) = axi_dma.rx_from_hw() {
            let buf = bufs[0].to_vec();
            bufs.clear();
            drop(bufs);
            drop(axi_dma);
            let rx_frame = Box::pin([0u8; PAYLOAD_SIZE]);
            AXI_DMA.lock().rx_submit(&[&rx_frame]);
            AXI_DMA.lock().rx_to_hw();
            Some(buf)
        } else {
            None
        }
    }

    pub fn recycle_rx_buffer(&self, buf: Vec<u8>) {
        drop(buf)
    }
    
}



lazy_static! {
    pub static ref ETHERNET: Arc<SpinLock<AxiEthernet>> = Arc::new(SpinLock::new(AxiEthernet::new(AXI_ETHERNET_BASE_ADDR, AXI_DMA_CONFIG.base_address)));
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
    // 打开地址过滤    
    ETHERNET.lock().enable_intr(XAE_INT_RECV_ERROR_MASK);
    ETHERNET.lock().start();
}





