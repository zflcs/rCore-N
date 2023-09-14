mod port_table;
mod tcp;
mod socket;

use spin::Mutex;
use alloc::{sync::Arc, collections::BTreeMap};
use lose_net_stack::{results::Packet, LoseStack, MacAddress, TcpFlags, IPv4};
use socket::{get_socket, push_data, get_s_a_by_index};
use port_table::check_accept;
use crate::driver::{net::NetDevice, dma::AXI_DMA_INTR};

pub use port_table::{accept, listen, port_acceptable, PortFd};
pub struct NetStack(Mutex<LoseStack>);



#[cfg(feature = "board_axu15eg")]
impl NetStack {
    pub fn new() -> Self {
        NetStack(Mutex::new(LoseStack::new(
            IPv4::new(192, 168, 1, 2),
            MacAddress::new([0x00, 0x0A, 0x35, 0x01, 0x02, 0x03]),
        )))
    }
}

lazy_static::lazy_static! {
    pub static ref LOSE_NET_STACK: Arc<NetStack> = Arc::new(NetStack::new());
    pub static ref ASYNC_RDMP: Arc<Mutex<BTreeMap<usize, usize>>> = Arc::new(Mutex::new(BTreeMap::new()));
}


#[cfg(feature = "board_axu15eg")]
pub fn net_interrupt_handler(irq: u16) {

    if irq == 4 {
        log::trace!("new mm2s intr");
        AXI_DMA_INTR.lock().tx_intr_handler();
    } else if irq == 5 {
        log::trace!("new s2mm intr");
        AXI_DMA_INTR.lock().rx_intr_handler();
        match NetDevice.receive() {
            Some(buf) => {
                let packet = LOSE_NET_STACK
                    .0
                    .lock()
                    .analysis(&buf);
                log::trace!("{:?}", packet);
                match packet {
                    Packet::ARP(arp_packet) => {
                        let lose_stack = LOSE_NET_STACK.0.lock();
                        if let Ok(reply_packet) = arp_packet.reply_packet(lose_stack.ip, lose_stack.mac) {
                            log::trace!("receive arp, need reply");
                            let reply_data = reply_packet.build_data();
                            NetDevice.transmit(&reply_data)
                        } else {
                            log::trace!("receive arp, do not need reply");
                        }
                    }
                    Packet::TCP(tcp_packet) => {
                        let target = tcp_packet.source_ip;
                        let lport = tcp_packet.dest_port;
                        let rport = tcp_packet.source_port;
                        let flags = tcp_packet.flags;
                        log::trace!("[TCP] target: {}, lport: {}, rport: {}", target, lport, rport);
                        if flags.contains(TcpFlags::S) {
                            // if it has a port to accept, then response the request
                            if check_accept(lport, &tcp_packet).is_some() {
                                let mut reply_packet = tcp_packet.ack();
                                reply_packet.flags = TcpFlags::S | TcpFlags::A;
                                NetDevice.transmit(&reply_packet.build_data());
                            } else {
                                log::error!("check accept failed");
                            }
                            NetDevice.recycle_rx_buffer(buf);
                            return;
                        } else if tcp_packet.flags.contains(TcpFlags::F) {
                            // tcp disconnected
                            let reply_packet = tcp_packet.ack();
                            NetDevice.transmit(&reply_packet.build_data());
                            let mut end_packet: lose_net_stack::packets::tcp::TCPPacket = reply_packet.ack();
                            end_packet.flags |= TcpFlags::F;
                            NetDevice.transmit(&end_packet.build_data());
                        } else if tcp_packet.flags.contains(TcpFlags::A) && tcp_packet.data_len == 0 {
                            let reply_packet = tcp_packet.ack();
                            NetDevice.transmit(&reply_packet.build_data());
                            NetDevice.recycle_rx_buffer(buf);
                            return;
                        } else {
                            let reply_packet = tcp_packet.ack();
                            NetDevice.transmit(&reply_packet.build_data());
                        }
                        if let Some(socket_index) = get_socket(target, lport, rport) {
                            let packet_seq = tcp_packet.seq;
                            if let Some((_seq, ack)) = get_s_a_by_index(socket_index) {
                                log::trace!("packet_seq: {}, ack: {}", packet_seq, ack);
                                if ack == packet_seq && tcp_packet.data_len > 0 {
                                    log::trace!("push data: {}, {}", socket_index, tcp_packet.data_len);
                                    push_data(socket_index, &tcp_packet);
                                }
                            }
                        }
                    }
                    _ => {
                        log::trace!("packet not match {:?}", packet);
                    }
                }
                NetDevice.recycle_rx_buffer(buf);
            }
            None => {
                log::trace!("do nothing");
            },
        }
    }
}