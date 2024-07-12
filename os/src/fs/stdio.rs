use super::File;
use crate::mm::UserBuffer;
use crate::print;
use crate::uart::{serial_getchar, serial_putchar};
use alloc::boxed::Box;
use core::fmt::{self, Write};
use core::{future::Future, pin::Pin};

pub struct Stdin;

pub struct Stdout;

impl File for Stdin {
    /// 在用户态的封装为 getchar（读取一个字符），所以 UserBuffer 的长度只支持 1
    fn read(&self, mut user_buf: UserBuffer) -> Result<usize, isize> {
        assert_eq!(user_buf.len(), 1);
        // busy loop
        if let Ok(ch) = serial_getchar(0) {
            unsafe {
                user_buf.buffers[0].as_mut_ptr().write_volatile(ch);
            }
            Ok(1)
        } else {
            Err(-1)
        }
    }
    fn write(&self, _user_buf: UserBuffer) -> Result<usize, isize> {
        panic!("Cannot write to stdin!");
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
        true
    }

    fn writable(&self) -> bool {
        false
    }
}

impl File for Stdout {
    fn read(&self, _user_buf: UserBuffer) -> Result<usize, isize> {
        panic!("Cannot read from stdout!");
    }
    fn write(&self, user_buf: UserBuffer) -> Result<usize, isize> {
        for buffer in user_buf.buffers.iter() {
            print!("{}", core::str::from_utf8(buffer).unwrap());
        }
        Ok(user_buf.len())
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
        false
    }

    fn writable(&self) -> bool {
        true
    }
}

impl Write for Stdout {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        for c in s.chars() {
            let _ = serial_putchar(0, c as u8);
        }
        Ok(())
    }
}

#[allow(dead_code)]
pub fn print(args: fmt::Arguments) {
    Stdout.write_fmt(args).unwrap();
}

#[macro_export]
macro_rules! print {
    ($fmt: literal $(, $($arg: tt)+)?) => {
        $crate::fs::stdio::print(format_args!($fmt $(, $($arg)+)?));
    }
}

#[macro_export]
macro_rules! println {
    ($fmt: literal $(, $($arg: tt)+)?) => {
        $crate::fs::stdio::print(format_args!(concat!($fmt, "\r\n") $(, $($arg)+)?));
    }
}
