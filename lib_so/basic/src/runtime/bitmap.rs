use bit_field::BitField;
use config::{PRIO_PTR, PRIO_NUM};
/// 协程优先级位图
#[derive(Clone, Copy)]
pub struct BitMap{
    pub ptr: usize
}

impl BitMap {
    pub const EMPTY: Self = Self {
        ptr: PRIO_PTR
    };

    pub fn update(&mut self, prio: usize, val: bool) {
        assert!(prio < PRIO_NUM);
        let ptr = unsafe { (self.ptr as *mut usize).as_mut().unwrap() };
        (*ptr).set_bit(prio, val);
    }
}