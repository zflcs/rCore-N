use crate::config::{CLOCK_FREQ, CPU_NUM};
use riscv::register::time;
use spin::Mutex;

const TICKS_PER_SEC: usize = 100;
const MSEC_PER_SEC: usize = 1000;
pub const USEC_PER_SEC: usize = 1_000_000;


#[repr(C)]
#[derive(Debug)]
pub struct TimeVal {
    pub sec: usize,
    pub usec: usize,
}

#[allow(dead_code)]
impl TimeVal {
    pub fn new() -> Self {
        TimeVal { sec: 0, usec: 0 }
    }
}


#[allow(unused)]
pub fn get_time_ms() -> usize {
    time::read() / (CLOCK_FREQ / MSEC_PER_SEC)
}

#[allow(unused)]
pub fn get_time_us() -> usize {
    time::read() * USEC_PER_SEC / CLOCK_FREQ
}

pub fn set_next_trigger() {
    sbi_rt::set_timer((time::read() + CLOCK_FREQ / TICKS_PER_SEC) as _);
}
