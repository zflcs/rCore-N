use config::MMIO;

use virtio_drivers::{
    device::net::VirtIONet,
    transport::{
        mmio::{MmioTransport, VirtIOHeader},
        DeviceType, Transport,
    },
};
use crate::device::virtio_impl::HalImpl;

const VIRT_NET_VADDR: usize = (*MMIO)[1].0;
const VIRT_NET_SIZE: usize = (*MMIO)[1].1;
pub const NET_BUFFER_LEN: usize = 2048;
pub const NET_QUEUE_SIZE: usize = 16;

pub fn init_net() {
    crate::device::walk_into_device(VIRT_NET_VADDR, VIRT_NET_SIZE);
}


pub fn virtio_net<T: Transport>(transport: T) {
    let net = VirtIONet::<HalImpl, T, NET_QUEUE_SIZE>::new(transport, NET_BUFFER_LEN)
        .expect("failed to create net driver");
    info!("MAC address: {:02x?}", net.mac_address());
    let mut net = net;
    loop {
        match net.receive() {
            Ok(buf) => {
                info!("RECV {} bytes: {:02x?}", buf.packet_len(), buf.packet());
                let tx_buf = virtio_drivers::device::net::TxBuffer::from(buf.packet());
                net.send(tx_buf).expect("failed to send");
                net.recycle_rx_buffer(buf).unwrap();
                break;
            }
            Err(virtio_drivers::Error::NotReady) => continue,
            Err(err) => panic!("failed to recv: {:?}", err),
        }
    }
    info!("virtio-net test finished");
    // super::tcp::test_echo_server(net);
}

