const MAX_USER_TRAP_NUM: usize = 128;

use crate::config::CPU_NUM;
use crate::plic::Plic;
use crate::task::{add_user_intr_task, hart_id, pid2process};
use crate::trace::{
    push_trace, DISABLE_USER_EXT_INT_ENTER, DISABLE_USER_EXT_INT_EXIT, ENABLE_USER_EXT_INT_ENTER,
    ENABLE_USER_EXT_INT_EXIT, PUSH_TRAP_RECORD_ENTER, PUSH_TRAP_RECORD_EXIT,
};
use crate::{mm::PhysPageNum, plic::get_context};
use alloc::{collections::BTreeMap, vec::Vec};
use core::arch::asm;
use heapless::spsc::Queue;
use lazy_static::*;
use spin::Mutex;

pub type UserTrapQueue = Queue<UserTrapRecord, MAX_USER_TRAP_NUM>;
#[derive(Clone)]
pub struct UserTrapInfo {
    pub user_trap_buffer_ppn: PhysPageNum,
    pub devices: Vec<(u16, bool)>,
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct UserTrapRecord {
    /// cause number 见 trace/analyze-trace.py
    ///
    /// cause_intr = {
    ///     0: "usi",
    ///     1: "ssi",
    ///     2: "hsi",
    ///     3: "msi",
    ///     4: "uti",
    ///     5: "sti",
    ///     6: "hti",
    ///     7: "mti",
    ///     8: "uei",
    ///     9: "sei",
    ///     10: "hei",
    ///     11: "mei",
    /// }
    pub cause: usize,
    /// cid / irq / get_time_us
    pub message: usize,
}

#[allow(dead_code)]
#[derive(Debug)]
pub enum UserTrapError {
    TaskNotFound,
    TrapUninitialized,
    TrapBufferFull,
    TrapThreadBusy,
}

impl UserTrapInfo {
    pub fn push_trap_record(&mut self, trap_record: UserTrapRecord) -> Result<(), UserTrapError> {
        let res = self.get_trap_queue_mut().enqueue(trap_record);
        match res {
            Ok(()) => Ok(()),
            Err(_) => {
                warn!("[push trap record] User TrapBufferFull!");
                Err(UserTrapError::TrapBufferFull)
            }
        }
    }

    pub fn enable_user_ext_int(&self) {
        push_trace(ENABLE_USER_EXT_INT_ENTER);

        let u_context = get_context(hart_id(), 'U');
        for (device_id, is_enabled) in &self.devices {
            for hart_id in 0..CPU_NUM {
                Plic::disable(get_context(hart_id, 'S'), *device_id);
            }
            if *is_enabled {
                Plic::enable(u_context, *device_id);
            } else {
                Plic::disable(u_context, *device_id);
            }
        }
        unsafe {
            asm!("fence iorw,iorw");
        }
        push_trace(ENABLE_USER_EXT_INT_EXIT);
    }

    pub fn disable_user_ext_int(&self) {
        push_trace(DISABLE_USER_EXT_INT_ENTER);

        let hart_id = hart_id();
        for (device_id, is_enabled) in &self.devices {
            Plic::disable(get_context(hart_id, 'U'), *device_id);
            if *is_enabled {
                Plic::enable(get_context(hart_id, 'S'), *device_id);
            } else {
                Plic::disable(get_context(hart_id, 'S'), *device_id);
            }
        }
        unsafe {
            asm!("fence iorw,iorw");
        }
        push_trace(DISABLE_USER_EXT_INT_EXIT);
    }

    pub fn remove_user_ext_int_map(&self) {
        let mut int_map = USER_EXT_INT_MAP.lock();
        for hart_id in 0..CPU_NUM {
            let s_context = get_context(hart_id, 'S');
            let u_context = get_context(hart_id, 'U');
            for (device_id, _) in &self.devices {
                // Plic::enable(u_context, *device_id);
                // Plic::claim(u_context);
                // Plic::complete(u_context, *device_id);
                Plic::disable(u_context, *device_id);
                Plic::enable(s_context, *device_id);
                Plic::complete(s_context, *device_id);
                int_map.remove(device_id);
            }
        }
    }

    pub fn get_trap_queue(&self) -> &UserTrapQueue {
        self.user_trap_buffer_ppn.get_mut::<UserTrapQueue>()
    }

    pub fn get_trap_queue_mut(&mut self) -> &mut UserTrapQueue {
        self.user_trap_buffer_ppn.get_mut::<UserTrapQueue>()
    }

    pub fn user_trap_record_num(&self) -> usize {
        self.get_trap_queue().len()
    }
}

lazy_static! {
    pub static ref USER_EXT_INT_MAP: Mutex<BTreeMap<u16, usize>> = Mutex::new(BTreeMap::new());
}

/// 将中断信息添加到任务调度程序和进程控制块
pub fn push_trap_record(pid: usize, trap_record: UserTrapRecord) -> Result<(), UserTrapError> {
    push_trace(PUSH_TRAP_RECORD_ENTER + pid);
    debug!(
        "[push trap record] pid: {}, cause: {}, message: {}",
        pid, trap_record.cause, trap_record.message
    );
    if let Some(pcb) = pid2process(pid) {
        let mut pcb_inner = pcb.acquire_inner_lock();
        if !pcb_inner.is_user_trap_enabled() {
            warn!("[push trap record] User trap disabled!");
            // return Err(UserTrapError::TrapDisabled);
        }
        // if let Some(trap_info) = &mut pcb_inner.user_trap_info {
        //     let mut res;
        //     let mut task = None;
        //     if pcb_inner.user_trap_handler_task.is_some() {
        //         task = pcb_inner.user_trap_handler_task.take();
        //     }
        //     drop(pcb_inner);
        //     if task.is_some() {
        //         res = trap_info.push_trap_record(trap_record);
        //         add_task(task.unwrap());
        //         add_user_intr_task(pid);
        //         debug!("wake handler task");
        //     }
        //     push_trace(PUSH_TRAP_RECORD_EXIT);
        //     res
        // } else {
        //     warn!("[push trap record] User trap uninitialized!");
        //     push_trace(PUSH_TRAP_RECORD_EXIT);
        //     Err(UserTrapError::TrapUninitialized)
        // }
        add_user_intr_task(pid); // 向任务池的调度程序添加这个进程的用户态中断
        pcb_inner.push_user_trap_record(trap_record) // 向进程控制块添加中断信息
    } else {
        warn!("[push trap record] Task Not Found!");
        push_trace(PUSH_TRAP_RECORD_EXIT);
        Err(UserTrapError::TaskNotFound)
    }
}
