use alloc::{sync::Arc, vec};
use smoltcp::{
    iface::{Interface, Config},
    wire::{EthernetAddress, IpCidr, IpAddress}, 
    time::Instant,
};
use spin::{Mutex, Lazy};
use smoltcp::iface::SocketSet;


use crate::device::NET_DEVICE;

pub fn set_up() {
    #[cfg(feature = "board_axu15eg")]
    INTERFACE.lock().update_ip_addrs(|ip_addrs|
        ip_addrs.push(IpCidr::new(IpAddress::v4(172, 16, 1, 2), 30)).unwrap()
    );

    #[cfg(feature = "board_qemu")]
    INTERFACE.lock().update_ip_addrs(|ip_addrs|
        ip_addrs.push(IpCidr::new(IpAddress::v4(10, 0, 2, 15), 30)).unwrap()
    );
}

pub fn iface_poll() {
    INTERFACE.lock().poll(
        Instant::ZERO, 
        unsafe { &mut *NET_DEVICE.as_mut_ptr() },
        &mut SOCKET_SET.lock()
    );
}


pub static INTERFACE: Lazy<Arc<Mutex<Interface>>> = Lazy::new(|| Arc::new(Mutex::new(
    Interface::new(
            Config::new(NET_DEVICE.mac().into()),
            unsafe { &mut *NET_DEVICE.as_mut_ptr() },
            Instant::ZERO
        )
)));



pub static SOCKET_SET: Lazy<Arc<Mutex<SocketSet>>> = Lazy::new(|| Arc::new(Mutex::new(
    SocketSet::new(vec![])
)));







