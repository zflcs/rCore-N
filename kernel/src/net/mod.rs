mod port_table;
mod tcp;
mod socket;
mod reply;

use spin::Lazy;

use crate::driver::{net::NetDevice, dma::AXI_DMA_INTR};

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


#[cfg(feature = "board_axu15eg")]
pub fn net_interrupt_handler(irq: u16) {
    use crate::{net::reply::{build_arp_repr, build_eth_repr, analysis_tcp, build_eth_frame}, driver::net::ETHERNET};
    if irq == 2 {
        log::debug!("new mac_irq");
    } else if irq == 3 {            // maybe need to wait a moment
        // log::debug!("new interrupt {:b}", ETHERNET.lock().get_intr_status());
        
        // ETHERNET.lock().clear_intr(mask);
    } else if irq == 4 {
        log::trace!("new mm2s intr");
        AXI_DMA_INTR.lock().tx_intr_handler();
        // AXI_DMA.lock().tx_from_hw(); 
    } else if irq == 5 {
        log::trace!("new s2mm intr");
        AXI_DMA_INTR.lock().rx_intr_handler();
        if let Some(buf) = NetDevice.receive() {
            if let Ok(mut eth_packet) = EthernetFrame::new_checked(buf) {
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
                                    NetDevice.transmit(eth_frame.as_ref());
                                }
                            } else {
                                log::trace!("don't need to reply")
                            }
                        } else {
                            log::warn!("Cannot analysis Arp protocol");
                        }
                    },
                    EthernetProtocol::Ipv4 => {
                        if let Ok(ipv4_packet) = Ipv4Packet::new_checked(eth_packet.payload_mut()) {
                            if ipv4_packet.next_header() == IpProtocol::Tcp {
                                if let Some(frames) = analysis_tcp(&mut eth_packet) {
                                    for eth_frame in frames {
                                        NetDevice.transmit(eth_frame.as_ref());
                                    }
                                }
                            } else {
                                log::warn!("Protocol based on IP is not supported");
                            }
                        }
                    },
                    _ => { log::warn!("Protocol is not supported"); }
                }
            }
        } else {
            log::trace!("do nothing");
        }
    }
}