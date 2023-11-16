use super::File;
use crate::mm::UserBuffer;

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