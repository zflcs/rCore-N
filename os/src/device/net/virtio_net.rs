use crate::device::virtio_bus::VirtioHal;
use alloc::sync::Arc;
use smoltcp::{
    phy::{Device, DeviceCapabilities, Medium, RxToken, TxToken},
    wire::EthernetAddress,
};
use spin::{Lazy, Mutex};
use virtio_drivers::{VirtIOHeader, VirtIONet};

static VIRTIO_NET_ADDR: usize = 0x10008000;

pub static NET_DEVICE: Lazy<NetDevice> = Lazy::new(|| NetDevice::new());

#[derive(Clone)]
pub struct NetDevice(Arc<Mutex<VirtIONet<'static, VirtioHal>>>);

impl NetDevice {
    pub fn new() -> Self {
        let virtio =
            VirtIONet::<VirtioHal>::new(unsafe { &mut *(VIRTIO_NET_ADDR as *mut VirtIOHeader) })
                .expect("can't create net device by virtio");
        Self(Arc::new(Mutex::new(virtio)))
    }

    pub fn mac(&self) -> EthernetAddress {
        EthernetAddress(self.0.lock().mac())
    }
}

impl RxToken for NetDevice {
    fn consume<R, F: FnOnce(&mut [u8]) -> R>(self, f: F) -> R {
        let mut buffer = [0u8; 2000];
        let mut driver = self.0.lock();
        let len = driver.recv(&mut buffer).expect("failed to recv packet");
        f(&mut buffer[..len])
    }
}

impl TxToken for NetDevice {
    fn consume<R, F: FnOnce(&mut [u8]) -> R>(self, len: usize, f: F) -> R {
        let mut buffer = [0u8; 2000];
        let result = f(&mut buffer[..len]);
        let mut driver = self.0.lock();
        driver.send(&buffer).expect("failed to send packet");
        result
    }
}

impl Device for NetDevice {
    type RxToken<'a> = Self;
    type TxToken<'a> = Self;

    fn receive(
        &mut self,
        _timestamp: smoltcp::time::Instant,
    ) -> Option<(Self::RxToken<'_>, Self::TxToken<'_>)> {
        let net = self.0.lock();
        if net.can_recv() {
            Some((self.clone(), self.clone()))
        } else {
            None
        }
    }

    fn transmit(&mut self, _timestamp: smoltcp::time::Instant) -> Option<Self::TxToken<'_>> {
        let net = self.0.lock();
        if net.can_send() {
            Some(self.clone())
        } else {
            None
        }
    }

    fn capabilities(&self) -> DeviceCapabilities {
        let mut caps = DeviceCapabilities::default();
        caps.max_transmission_unit = 1536;
        caps.max_burst_size = Some(1);
        caps.medium = Medium::Ethernet;
        caps
    }
}

pub fn init() {}
