use core::fmt::{self, Write};
use spin::{Lazy, Mutex};
use user_syscall::{read, write};

struct StdIn(usize);

struct StdOut(usize);

impl StdIn {
    pub const STDIN: Self = Self(0);

    pub fn getchar(&self) -> u8 {
        let mut c = [0u8; 1];
        read(self.0, &mut c);
        c[0]
    }
}

impl StdOut {
    pub const STDOUT: Self = Self(1);
}

impl Write for StdOut {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        write(self.0, s.as_bytes());
        Ok(())
    }
}


static STDIN: Lazy<Mutex<StdIn>> = Lazy::new(|| Mutex::new(StdIn::STDIN));
static STDOUT: Lazy<Mutex<StdOut>> = Lazy::new(|| Mutex::new(StdOut::STDOUT));



pub fn print(args: fmt::Arguments) {
    STDOUT.lock().write_fmt(args).unwrap();
}

#[macro_export]
macro_rules! print {
    ($fmt: literal $(, $($arg: tt)+)?) => {
        $crate::console::print(format_args!($fmt $(, $($arg)+)?));
    }
}

#[macro_export]
macro_rules! println {
    ($fmt: literal $(, $($arg: tt)+)?) => {
        $crate::console::print(format_args!(concat!($fmt, "\n") $(, $($arg)+)?));
    }
}

pub fn getchar() -> u8 {
    STDIN.lock().getchar()
}