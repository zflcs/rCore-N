use crate::net::net_interrupt_handler;
use rv_plic::{Priority, PLIC};

pub const PLIC_BASE: usize = 0xc00_0000;
pub const PLIC_PRIORITY_BIT: usize = 3;

pub type Plic = PLIC<{ PLIC_BASE }, { PLIC_PRIORITY_BIT }>;

pub fn get_context(hart_id: usize, mode: char) -> usize {
    const MODE_PER_HART: usize = 3;
    hart_id * MODE_PER_HART
        + match mode {
            'M' => 0,
            'S' => 1,
            'U' => 2,
            _ => panic!("Wrong Mode"),
        }
}

#[cfg(feature = "board_qemu")]
pub fn init() {
    Plic::set_priority(8, Priority::lowest());
}

#[cfg(feature = "board_axu15eg")]
pub fn init() {
    for i in 1..=6 {
        Plic::set_priority(i, Priority::lowest());
    }
}

#[cfg(feature = "board_qemu")]
pub fn init_hart(hart_id: usize) {
    let context = get_context(hart_id, 'S');
    Plic::enable(context, 8);
    Plic::set_threshold(context, Priority::any());
}

#[cfg(feature = "board_axu15eg")]
pub fn init_hart(hart_id: usize) {
    let context = get_context(hart_id, 'S');
    Plic::clear_enable(context, 0);
    Plic::clear_enable(get_context(hart_id, 'U'), 0);
    Plic::enable(context, 1);
    Plic::enable(context, 2);
    Plic::enable(context, 3);
    // Plic::enable(context, 4);
    // Plic::enable(context, 5);
    Plic::set_threshold(context, Priority::any());
    Plic::set_threshold(get_context(hart_id, 'U'), Priority::any());
    Plic::set_threshold(get_context(hart_id, 'M'), Priority::never());
}

pub fn handle_external_interrupt(hart_id: usize) {
    let context = get_context(hart_id, 'S');
    while let Some(irq) = Plic::claim(context) {
        match irq {
            #[cfg(feature = "board_qemu")]
            8 => {
                net_interrupt_handler(irq);
            }
            #[cfg(feature = "board_axu15eg")]
            2 | 3 | 4 | 5 => {
                net_interrupt_handler(irq);
            }
            _ => {
                warn!("[PLIC]: irq {:?} not supported!", irq);
            }
        }
        Plic::complete(context, irq);
    }
}
