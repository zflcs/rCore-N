mod iface;
mod tcp;

pub use iface::*;
pub use tcp::TcpFile;

pub fn init() {
    iface::set_up();
}

#[cfg(feature = "board_axu15eg")]
pub fn net_interrupt_handler(irq: u16) {
    if irq == 2 {
        log::debug!("new mac_irq");
    } else if irq == 3 {
        if NET_DEVICE.eth.lock().is_rx_cmplt() {
            iface::iface_poll();
        } else if NET_DEVICE.eth.lock().is_tx_cmplt() {
            NET_DEVICE.eth.lock().clear_tx_cmplt();
        } else {
            // log::warn!("other interrupt {:b} happend", NET_DEVICE.eth.lock().get_intr_status());
        }
    }
}

#[cfg(feature = "board_qemu")]
pub fn net_interrupt_handler(irq: u16) {
    if irq == 8 {
        log::debug!("new net interrupt");
        iface::iface_poll();
    }
}
