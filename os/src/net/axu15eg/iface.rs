use alloc::{sync::Arc, vec};
use kernel_sync::SpinLock;
use smoltcp::{
    iface::{Interface, Config},
    wire::{EthernetAddress, IpCidr, IpAddress}, 
    time::Instant,
};
use spin::Lazy;
use smoltcp::iface::SocketSet;


use crate::{config::AXI_NET_CONFIG, device::NET_DEVICE};

pub fn set_up() {
    INTERFACE.lock().update_ip_addrs(|ip_addrs|
        ip_addrs.push(IpCidr::new(IpAddress::v4(172, 16, 1, 2), 30)).unwrap()
    );
}

pub fn iface_poll() {
    INTERFACE.lock().poll(
        Instant::ZERO, 
        unsafe { &mut *NET_DEVICE.as_mut_ptr() },
        &mut SOCKET_SET.lock()
    );
}


pub static INTERFACE: Lazy<Arc<SpinLock<Interface>>> = Lazy::new(|| Arc::new(SpinLock::new(
    Interface::new(
            Config::new(EthernetAddress(AXI_NET_CONFIG.mac_addr).into()),
            unsafe { &mut *NET_DEVICE.as_mut_ptr() },
            Instant::ZERO
        )
)));



pub static SOCKET_SET: Lazy<Arc<SpinLock<SocketSet>>> = Lazy::new(|| Arc::new(SpinLock::new(
    SocketSet::new(vec![])
)));







