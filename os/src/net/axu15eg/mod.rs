mod port_table;
mod tcp;
mod socket;
mod reply;

use alloc::{sync::Arc, collections::BTreeMap};
use spin::{Lazy, Mutex};

use crate::device::net::{NET_DEVICE, AXI_DMA_INTR};

pub use port_table::{accept, listen, port_acceptable, PortFd};
use smoltcp::wire::*;

#[cfg(feature = "board_axu15eg")]
pub struct NetStack {
    mac_addr: EthernetAddress,
    ipv4_addr: Ipv4Address,
}

#[cfg(feature = "board_axu15eg")]
impl Default for NetStack {
    fn default() -> Self {
        Self { 
            mac_addr: EthernetAddress::from_bytes(&[0x00, 0x0A, 0x35, 0x01, 0x02, 0x03]),
            ipv4_addr: Ipv4Address::new(172, 16, 1, 2),
        }
    }
}

#[cfg(feature = "board_axu15eg")]
pub static NET_STACK: Lazy<NetStack> = Lazy::new(|| NetStack::default());
pub static ASYNC_RDMP: Lazy<Arc<Mutex<BTreeMap<usize, usize>>>> = Lazy::new(|| Arc::new(Mutex::new(BTreeMap::new())));


#[cfg(feature = "board_axu15eg")]
pub fn net_interrupt_handler(irq: u16) {
    use crate::{net::axu15eg::reply::{build_arp_repr, build_eth_repr, analysis_tcp, build_eth_frame}, device::net::{NET_DEVICE, self},};
    if irq == 2 {
        log::debug!("new mac_irq");
    } else if irq == 3 {            // maybe need to wait a moment
        log::trace!("new interrupt {:b}", NET_DEVICE.eth.lock().get_intr_status());
        if NET_DEVICE.eth.lock().is_rx_cmplt() {
            while let Some(mut buf) = NET_DEVICE.receive() {
                if let Ok(mut eth_packet) = EthernetFrame::new_checked(&mut *buf) {
                    match eth_packet.ethertype() {
                        EthernetProtocol::Arp => {
                            if let Ok(arp_packet) = ArpPacket::new_checked(eth_packet.payload_mut()) {
                                if arp_packet.operation() == ArpOperation::Request {
                                    let dst_mac_addr = EthernetAddress::from_bytes(arp_packet.source_hardware_addr());
                                    let arp_repr = build_arp_repr(
                                        NET_STACK.mac_addr, 
                                        NET_STACK.ipv4_addr, 
                                        dst_mac_addr,
                                        Ipv4Address::from_bytes(arp_packet.source_protocol_addr())    
                                    );
                                    let eth_repr = build_eth_repr(
                                        NET_STACK.mac_addr, 
                                        dst_mac_addr, 
                                        EthernetProtocol::Arp
                                    );
                                    if let Some(eth_frame) = build_eth_frame(eth_repr, Some(arp_repr), None) {
                                        NET_DEVICE.transmit(eth_frame.as_ref());
                                    }
                                } else {
                                    log::trace!("don't need to reply")
                                }
                            } else {
                                log::trace!("Cannot analysis Arp protocol");
                            }
                        },
                        EthernetProtocol::Ipv4 => {
                            if let Ok(ipv4_packet) = Ipv4Packet::new_checked(eth_packet.payload_mut()) {
                                if ipv4_packet.next_header() == IpProtocol::Tcp {
                                    if let Some(frames) = analysis_tcp(&mut eth_packet) {
                                        for eth_frame in frames {
                                            NET_DEVICE.transmit(eth_frame.as_ref());
                                        }
                                    }
                                } else {
                                    log::trace!("Protocol based on IP is not supported");
                                }
                            }
                        },
                        _ => { log::trace!("Protocol is not supported"); }
                    }
                }
            }
        } else {
            log::warn!("other interrupt happend");
        }
    } 
}