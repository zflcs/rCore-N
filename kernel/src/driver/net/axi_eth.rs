


use alloc::{sync::Arc, vec::Vec};
use axi_ethernet::*;
use lazy_static::lazy_static;
use kernel_sync::SpinLock;
use log::trace;

use crate::driver::dma::AXI_DMA;

pub const AXI_ETHERNET_BASE_ADDR: usize = 0x60140000;
pub const MAC_ADDR: [u8; 6] = [0x00, 0x0A, 0x35, 0x01, 0x02, 0x03];

pub struct NetDevice;
use axidma::AXI_DMA_CONFIG;

impl NetDevice {

    // data 中需要包含源地址、目的地址、eth type/len 信息
    #[allow(unused)]
    fn fill_frame(&self, tx_frame: &mut [u8], data: &[u8]) {
        log::trace!("fill tx frame");
        // fill payload
        let payload_size: usize = data.len();
        tx_frame[0..0 + payload_size].copy_from_slice(data);
        log::trace!("fill tx frame success");
    }

    pub fn transmit(&self, data: &[u8]) {
        log::trace!("net transmit");
        // reclaim tx descriptor block
        // AXI_DMA.lock().tx_from_hw();
        // 初始化填充发送帧
        AXI_DMA.lock().tx_submit(data);
        AXI_DMA.lock().tx_to_hw();    
    }

    pub fn receive(&self) -> Option<Vec<u8>> {
        // 将数据复制到 buffer 中，这里只从 buf0 读取数据，暂时没有用到 dma 的更多功能
        let mut axi_dma = AXI_DMA.lock();
        if let Some(bufs) = axi_dma.rx_from_hw() {
            let buf = bufs[0].to_vec();
            drop(bufs);
            drop(axi_dma);
            AXI_DMA.lock().rx_submit();
            AXI_DMA.lock().rx_to_hw();
            Some(buf)
        } else {
            None
        }
    }
    
}



lazy_static! {
    pub static ref ETHERNET: Arc<SpinLock<AxiEthernet>> = Arc::new(SpinLock::new(AxiEthernet::new(AXI_ETHERNET_BASE_ADDR, AXI_DMA_CONFIG.base_address)));
}

pub fn init() {
    #[allow(unused_assignments)]
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
    ETHERNET.lock().set_mac_address(&MAC_ADDR);
    trace!("link_status: {:?}", ETHERNET.lock().link_status);
    for _ in 0..100000 {}
    ETHERNET.lock().enable_intr(XAE_INT_RECV_ERROR_MASK);
    ETHERNET.lock().start();
}





