
use crate::net::NET_STACK;
use crate::net::axu15eg::port_table::check_accept;
use crate::net::axu15eg::socket::{get_socket, get_s_a_by_index, push_data};

pub(crate) const ARP_LEN: usize = 28;

use alloc::{vec, vec::Vec};
use smoltcp::wire::*;
use smoltcp::phy::ChecksumCapabilities;


pub fn build_eth_repr(src_mac_addr: EthernetAddress, dst_mac_addr: EthernetAddress, ethertype: EthernetProtocol) -> EthernetRepr {
    EthernetRepr {
        src_addr: src_mac_addr,
        dst_addr: dst_mac_addr,
        ethertype,
    }
}

pub fn build_arp_repr(src_mac_addr: EthernetAddress, src_ip: Ipv4Address, dst_mac_addr: EthernetAddress, dst_ip: Ipv4Address) -> ArpRepr {
    ArpRepr::EthernetIpv4 { 
        operation: ArpOperation::Reply, 
        source_hardware_addr: src_mac_addr, 
        source_protocol_addr: src_ip, 
        target_hardware_addr: dst_mac_addr, 
        target_protocol_addr: dst_ip, 
    }
}

pub fn build_ipv4_repr(src_ip: Ipv4Address, dst_ip: Ipv4Address, protocol: IpProtocol, payload_len: usize) -> Ipv4Repr {
    Ipv4Repr {
        src_addr: src_ip,
        dst_addr: dst_ip,
        next_header: protocol,
        payload_len,
        hop_limit: 128,
    }
}

// generate the tcp repr from the receive TcpPacket
pub fn build_tcp_ack_repr<'a>(packet: &TcpPacket<&'a [u8]>) -> TcpRepr<'a> {
    pub(crate) const TCP_EMPTY_DATA: &[u8] = &[];
    let src_port = packet.dst_port();
    let dst_port = packet.src_port();
    let control = if packet.syn() {
        TcpControl::Syn
    } else if packet.fin() {
        TcpControl::Fin
    } else {
        TcpControl::None
    };
    let ack_number = packet.seq_number() + packet.segment_len();
    let seq_number = if packet.ack() {
        packet.ack_number()
    } else {
        TcpSeqNumber::default()
    };
    TcpRepr {
        src_port,
        dst_port,
        control,
        seq_number,
        ack_number: Some(ack_number),
        window_len: packet.window_len(),
        window_scale: Some(8),
        max_seg_size: Some(1460),
        sack_permitted: false,
        sack_ranges: [None; 3],
        payload: TCP_EMPTY_DATA,
    }
}

pub fn build_tcp_repr<'a>(
    src_port: u16, 
    dst_port: u16, 
    control: TcpControl, 
    seq_number: TcpSeqNumber, 
    ack_number: Option<TcpSeqNumber>,
    payload: &'a [u8]
) -> TcpRepr<'a> {
    TcpRepr {
        src_port,
        dst_port,
        control,
        seq_number,
        ack_number,
        window_len: 1460,
        window_scale: None,
        max_seg_size: Some(1460),
        sack_permitted: false,
        sack_ranges: [None; 3],
        payload,
    }
}

pub fn build_eth_frame(eth_repr: EthernetRepr, arp_repr: Option<ArpRepr>, ipv4_repr: Option<(Ipv4Repr, TcpRepr)>) -> Option<EthernetFrame<Vec<u8>>> {
    if let Some(arp_repr) = arp_repr {
        let mut buf = vec![0u8; ETHERNET_HEADER_LEN + ARP_LEN];
        let mut arp_packet = ArpPacket::new_checked(&mut buf[ETHERNET_HEADER_LEN..]).unwrap();
        arp_repr.emit(&mut arp_packet);
        let mut eth_frame = EthernetFrame::new_checked(buf).unwrap();
        eth_repr.emit(&mut eth_frame);
        Some(eth_frame)

    } else {
        if let Some((ipv4_repr, tcp_repr)) = ipv4_repr {
            let checksum_ability = ChecksumCapabilities::default();
            let mut buf = vec![0u8; ETHERNET_HEADER_LEN + IPV4_HEADER_LEN + tcp_repr.buffer_len()];

            let mut tcp_packet = TcpPacket::new_unchecked(&mut buf[ETHERNET_HEADER_LEN + IPV4_HEADER_LEN..]);
            tcp_repr.emit(&mut tcp_packet, &ipv4_repr.src_addr.into_address(), &ipv4_repr.dst_addr.into_address(), &checksum_ability);

            let mut ipv4_packet = Ipv4Packet::new_checked(&mut buf[ETHERNET_HEADER_LEN..]).unwrap();
            ipv4_repr.emit(&mut ipv4_packet, &checksum_ability);

            let mut eth_frame = EthernetFrame::new_checked(buf).unwrap();
            eth_repr.emit(&mut eth_frame);
            Some(eth_frame)
        } else {
            None
        }
    }
}


pub fn analysis_tcp(eth_frame: &mut EthernetFrame<Vec<u8>>) -> Option<Vec<EthernetFrame<Vec<u8>>>> {
    assert!(eth_frame.ethertype() == EthernetProtocol::Ipv4);
    let src_mac_addr = NET_STACK.mac_addr;
    let dst_mac_addr = eth_frame.src_addr().clone();
    let binding = eth_frame.payload_mut();
    let ipv4_packet = Ipv4Packet::new_checked(binding.as_ref()).unwrap();
    assert!(ipv4_packet.next_header() == IpProtocol::Tcp);
    let src_ip = ipv4_packet.dst_addr();
    let dst_ip = ipv4_packet.src_addr();
    
    let packet = TcpPacket::new_checked(ipv4_packet.payload()).unwrap();
    if packet.syn() {   // send syn packet to the other one, only need to send 1 packet
        if check_accept(dst_mac_addr, dst_ip, src_ip, &packet) {
            let tcp_repr = build_tcp_ack_repr(&packet);
            let ipv4_repr = build_ipv4_repr(src_ip, dst_ip, IpProtocol::Tcp, tcp_repr.buffer_len());
            let eth_repr = build_eth_repr(src_mac_addr, dst_mac_addr, EthernetProtocol::Ipv4);
            if let Some(eth_frame) = build_eth_frame(eth_repr, None, Some((ipv4_repr, tcp_repr))) {
                Some(vec![eth_frame])
            } else {
                None
            }
        } else {
            None
        }
    } else if packet.fin() {    // close connection, need send 2 packet, but I make it send 1 packet, merge the Ack and Fin
        let tcp_repr = build_tcp_ack_repr(&packet);
        let ipv4_repr = build_ipv4_repr(src_ip, dst_ip, IpProtocol::Tcp, tcp_repr.buffer_len());
        let eth_repr = build_eth_repr(src_mac_addr, dst_mac_addr, EthernetProtocol::Ipv4);
        if let Some(eth_frame) = build_eth_frame(eth_repr, None, Some((ipv4_repr, tcp_repr))) {
            Some(vec![eth_frame])
        } else {
            None
        }
    } else if packet.psh() {    // need to push data to buf, need to send reply
        log::trace!("get psh packet");
        let tcp_repr = build_tcp_ack_repr(&packet);
        let ipv4_repr = build_ipv4_repr(src_ip, dst_ip, IpProtocol::Tcp, tcp_repr.buffer_len());
        let eth_repr = build_eth_repr(src_mac_addr, dst_mac_addr, EthernetProtocol::Ipv4);
        let eth_frames = if let Some(eth_frame) = build_eth_frame(eth_repr, None, Some((ipv4_repr, tcp_repr))) {
            Some(vec![eth_frame])
        } else {
            None
        };
        let lport = packet.src_port();
        let rport = packet.dst_port();
        if let Some(socket_index) = get_socket(dst_ip, lport, rport) {
            log::trace!("get socket index {}", socket_index);
            let packet_seq = packet.seq_number();
            if let Some((_, ack)) = get_s_a_by_index(socket_index) {
                log::trace!("packet_seq: {}, ack: {:?}", packet_seq, ack);
                if ack.unwrap() == packet_seq && packet.payload().len() > 0 {
                    log::debug!("push data: {}, {}", socket_index, packet.payload().len());
                    push_data(socket_index, &packet);
                }
                eth_frames
            } else {
                None
            }
        } else {
            None
        }
    } else {                    // don't need to send reply, usually is ack packet, not supported rst flag
        None
    }
}