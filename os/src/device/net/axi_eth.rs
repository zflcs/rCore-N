
use core::pin::Pin;

use alloc::{sync::Arc, boxed::Box, vec};
use axi_dma::{AxiDmaIntr, AxiDma};
use axi_ethernet::{AxiEthernet, XAE_JUMBO_OPTION, LinkStatus};
use kernel_sync::SpinLock;
use spin::Lazy;
use crate::config::{AXI_DMA_CONFIG, AXI_NET_CONFIG};
use smoltcp::phy::{Device, RxToken, TxToken, DeviceCapabilities, Medium};

#[derive(Clone)]
pub struct NetDevice {
    pub dma: Arc<AxiDma>,
    pub dma_intr: Arc<AxiDmaIntr>,
    pub eth: Arc<SpinLock<AxiEthernet>>,
}

impl NetDevice {
    pub const fn new(
        dma: Arc<AxiDma>,
        dma_intr: Arc<AxiDmaIntr>,
        eth: Arc<SpinLock<AxiEthernet>>
    ) -> Self {
        Self { dma, dma_intr, eth }
    }
}

impl Default for NetDevice {
    fn default() -> Self {
        NetDevice::new(AXI_DMA.clone(), AXI_DMA_INTR.clone(), AXI_ETH.clone())
    }
}

impl NetDevice {
    pub fn transmit(&self, data: &[u8]) {
        log::trace!("net transmit");
        let buf = self.dma.tx_submit(Box::pin(data)).unwrap().wait();
        if !self.dma_intr.tx_intr_handler() {
            dma_init();
        }
        self.dma.tx_from_hw();
    }

    pub fn receive(&self) -> Option<Pin<Box<[u8]>>> {
        let mut eth = self.eth.lock();
        if eth.is_rx_cmplt() {
            eth.clear_rx_cmplt();
        }
        if eth.can_receive() {
            let rx_frame = Box::pin([0u8; AXI_NET_CONFIG.mtu]);
            let mut buf = self.dma.rx_submit(rx_frame).unwrap().wait();
            if !self.dma_intr.rx_intr_handler() {
                dma_init();
            }
            self.dma.rx_from_hw();
            Some(buf)
        } else {
            None
        }
    }
}

pub static NET_DEVICE: Lazy<NetDevice> = Lazy::new(|| NetDevice::default());

pub static AXI_ETH: Lazy<Arc<SpinLock<AxiEthernet>>> = Lazy::new(||  Arc::new(SpinLock::new(AxiEthernet::new(
    AXI_NET_CONFIG.eth_baseaddr, AXI_NET_CONFIG.dma_baseaddr
))));

pub static AXI_DMA_INTR: Lazy<Arc<AxiDmaIntr>> = Lazy::new(|| AxiDmaIntr::new(AXI_DMA_CONFIG.base_address));

pub static AXI_DMA: Lazy<Arc<AxiDma>> = Lazy::new(|| AxiDma::new(AXI_DMA_CONFIG));


pub fn init() {
    dma_init();
    eth_init();
}

pub fn dma_init() {
    AXI_DMA.reset();
    // enable cyclic mode
    AXI_DMA.tx_cyclic_enable();
    AXI_DMA.rx_cyclic_enable();

    // init cyclic block descriptor
    AXI_DMA.tx_bd_create(AXI_NET_CONFIG.tx_bd_cnt);
    AXI_DMA.rx_bd_create(AXI_NET_CONFIG.rx_bd_cnt);

    // enable tx & rx intr
    AXI_DMA.tx_intr_enable();
    AXI_DMA.rx_intr_enable();
}

pub fn eth_init() {
    let mut eth = AXI_ETH.lock();
    eth.reset();
    let options = eth.get_options();
    eth.set_options(options | XAE_JUMBO_OPTION);
    eth.detect_phy();
    let speed = eth.get_phy_speed_ksz9031();
    debug!("speed is: {}", speed);
    eth.set_operating_speed(speed as u16);
    if speed == 0 {
        eth.link_status = LinkStatus::EthLinkDown;
    } else {
        eth.link_status = LinkStatus::EthLinkUp;
    }
    eth.set_mac_address(&AXI_NET_CONFIG.mac_addr);
    debug!("link_status: {:?}", eth.link_status);
    eth.enable_rx_memovr();
    eth.enable_rx_rject();
    eth.enable_rx_cmplt();
    eth.start();
}