
use core::{
    fmt::{Arguments, Write},
    str::FromStr,
};
use spin::{Lazy, Mutex};

use crate::hart_id;

/// 
pub extern crate log;

extern "C" {
    fn put_str(ptr: *const u8, len: usize);
}


/// init
pub fn init(env: Option<&str>) {
    log::set_logger(&Console).unwrap();
    set_log_level(env);
}

/// log level
pub fn set_log_level(env: Option<&str>) {
    use log::LevelFilter as Lv;
    log::set_max_level(env.and_then(|s| Lv::from_str(s).ok()).unwrap_or(Lv::Trace));
    // log::set_max_level(Lv::max());
}



/// _print
#[doc(hidden)]
#[inline]
pub fn _print(args: Arguments) {
    CONSOLE.lock().write_fmt(args).unwrap();
}



/// print!
#[macro_export]
macro_rules! print {
    ($fmt: literal $(, $($arg: tt)+)?) => {
        $crate::console::_print(format_args!($fmt $(, $($arg)+)?))
    }
}

/// println!
#[macro_export]
macro_rules! println {
    ($fmt: literal $(, $($arg: tt)+)?) => {
        $crate::console::_print(format_args!(concat!($fmt, "\n") $(, $($arg)+)?))
    }
}

///
static CONSOLE: Lazy<Mutex<Console>> = Lazy::new(|| Mutex::new(Console));

/// 
struct Console;

/// The requirement of `core::fmt::Write` trait
impl Write for Console {
    #[inline]
    fn write_str(&mut self, s: &str) -> Result<(), core::fmt::Error> {
        let buf = s.as_bytes();
        unsafe { put_str(buf.as_ptr(), buf.len()) }
        Ok(())
    }
}

/// The requirement of `log::Log` trait
impl log::Log for Console {
    fn enabled(&self, _metadata: &log::Metadata) -> bool {
        true
    }

    fn log(&self, record: &log::Record) {
        if !self.enabled(record.metadata()) {
            return;
        }
        use log::Level::*;
        let color_code: u8 = match record.level() {
            Error => 31,
            Warn => 93,
            Info => 34,
            Debug => 32,
            Trace => 90,
        };
        println!(
            "\x1b[{}m[core {}][{:>5}] {}:{} {}\x1b[0m",
            color_code,
            hart_id(),
            record.level(),
            record.file().unwrap(),
            record.line().unwrap(),
            record.args(),
        );
    }

    fn flush(&self) {}
}
