use bit_field::BitField;
use spin::Mutex;
use crate::config::PRIO_NUM;

/// 协程优先级位图，在用户态进行更新，在内核态只会读取
pub struct  BitMap{
    pub bits: usize,
    pub lock: Mutex<()>,
}

impl BitMap {
    pub const fn new() -> Self {
        Self{
            bits: 0,
            lock: Mutex::new(()),
        }
    }
    /// 更新
    pub fn update(&mut self, prio: usize, val: bool) {
        assert!(prio < PRIO_NUM);
        let lock = self.lock.lock();
        self.bits.set_bit(prio, val);
        drop(lock);
    }
    /// 
    pub fn get_val(&self) -> usize {
        self.bits
    }
}