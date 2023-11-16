use super::File;
use crate::mm::UserBuffer;
use crate::print;
use core::fmt::{self, Write};

pub struct Stdin;

pub struct Stdout;

impl File for Stdin {
    fn read(&self, mut user_buf: UserBuffer) -> Result<usize, isize> {
        assert_eq!(user_buf.len(), 1);
        let ch = crate::sbi::console_getchar() as isize;
        if ch < 0 {
            Err(-1)
        } else {
            unsafe { user_buf.buffers[0].as_mut_ptr().write_volatile(ch as _) };
            Ok(user_buf.len())
        }
    }
    fn write(&self, _user_buf: UserBuffer) -> Result<usize, isize> {
        panic!("Cannot write to stdin!");
    }
    fn awrite(&self, _buf: UserBuffer, _pid: usize, _key: usize) -> Result<usize, isize> {
        unimplemented!();
    }
    fn aread(&self, _buf: UserBuffer, _cid: usize, _pid: usize, _key: usize) -> Result<usize, isize> {
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
            print!("{}", core::str::from_utf8(*buffer).unwrap());
        }
        Ok(user_buf.len())
    }
    
    fn awrite(&self, _buf: UserBuffer, _pid: usize, _key: usize) -> Result<usize, isize> {
        unimplemented!();
    }
    
    fn aread(&self, _buf: UserBuffer, _cid: usize, _pid: usize, _key: usize) -> Result<usize, isize> {
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
            // let _ = serial_putchar(0, c as u8);
            let _ = crate::sbi::console_putchar(c as usize);
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
