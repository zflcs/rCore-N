mod virtio_impl;
mod net;

use core::ptr::NonNull;
use virtio_drivers::{
    device::net::VirtIONet,
    transport::{
        mmio::{MmioTransport, VirtIOHeader},
        DeviceType, Transport,
    },
};

use virtio_impl::HalImpl;
use net::{init_net, virtio_net};


pub fn init_dt() {
    init_net();
}

pub fn walk_into_device(vaddr: usize, size: usize) {
    info!("walk dt addr={:#x}, size={:#x}", vaddr, size);
    let header = NonNull::new(vaddr as *mut VirtIOHeader).unwrap();
    match unsafe { MmioTransport::new(header) } {
        Err(e) => warn!("Error creating VirtIO MMIO transport: {}", e),
        Ok(transport) => {
            info!(
                "Detected virtio MMIO device with vendor id {:#X}, device type {:?}, version {:?}",
                transport.vendor_id(),
                transport.device_type(),
                transport.version(),
            );
            virtio_device(transport);
        }
    }
}

fn virtio_device(transport: impl Transport) {
    match transport.device_type() {
        DeviceType::Network => virtio_net(transport),
        t => warn!("Unrecognized virtio device: {:?}", t),
    }
}




