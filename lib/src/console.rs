//! This crate provide `print!, `println!` and `log::Log`。

#![deny(warnings, missing_docs)]

use core::fmt::{Arguments, Write};


extern "C" {
    fn put_str(s: *const u8, len: usize);
}


/// _print
#[doc(hidden)]
#[inline]
pub fn _print(args: Arguments) {
    Console.write_fmt(args).unwrap();
}

/// print!
#[macro_export]
macro_rules! print {
    ($($arg:tt)*) => {
        $crate::console::_print(core::format_args!($($arg)*));
    }
}

/// println!
#[macro_export]
macro_rules! println {
    () => ($crate::print!("\n"));
    ($($arg:tt)*) => {{
        $crate::console::_print(core::format_args!($($arg)*));
        $crate::println!();
    }}
}

/// 
struct Console;

/// The requirement of `core::fmt::Write` trait
impl Write for Console {
    #[inline]
    fn write_str(&mut self, s: &str) -> Result<(), core::fmt::Error> {
        unsafe { put_str(s.as_ptr(), s.len()) };
        Ok(())
    }
}
