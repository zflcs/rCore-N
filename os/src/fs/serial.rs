use super::File;
use crate::mm::UserBuffer;
use crate::uart::{serial_getchar, serial_putchar};
use alloc::boxed::Box;
use core::{future::Future, pin::Pin};

pub struct Serial<const N: usize>;

impl<const N: usize> File for Serial<N> {
    fn read(&self, user_buf: UserBuffer) -> Result<usize, isize> {
        let mut read_cnt = 0;
        for ptr in user_buf {
            if let Ok(ch) = serial_getchar(N) {
                // debug!("Serial {} read: {}", N, ch);
                unsafe {
                    ptr.write_volatile(ch);
                }
                read_cnt += 1;
            } else {
                break;
            }
        }
        // debug!("Serial {} read cnt: {}", N, read_cnt);
        if read_cnt > 0 {
            Ok(read_cnt)
        } else {
            Err(-1)
        }
    }
    fn write(&self, user_buf: UserBuffer) -> Result<usize, isize> {
        let mut write_cnt = 0;
        let mut write_ok = true;
        for buffer in user_buf.buffers.iter() {
            for char in buffer.iter() {
                // debug!("Serial {} write: {}", N, *char);
                if let Ok(()) = serial_putchar(N, *char) {
                    write_cnt += 1;
                } else {
                    write_ok = false;
                    break;
                }
            }
            if !write_ok {
                break;
            }
        }
        if write_cnt > 0 {
            Ok(write_cnt)
        } else {
            Err(-1)
        }
    }
    fn awrite(
        &self,
        _buf: UserBuffer,
        _pid: usize,
        _key: usize,
    ) -> Pin<Box<dyn Future<Output = ()> + 'static + Send + Sync>> {
        unimplemented!();
    }
    fn aread(
        &self,
        _buf: UserBuffer,
        _cid: usize,
        _pid: usize,
        _key: usize,
    ) -> Pin<Box<dyn Future<Output = ()> + 'static + Send + Sync>> {
        unimplemented!();
    }

    fn readable(&self) -> bool {
        todo!()
    }

    fn writable(&self) -> bool {
        todo!()
    }
}
