use core::fmt::{Error, Write};

use crate::syscall::serial_write;

struct WriteImpl;

impl Write for WriteImpl {
    fn write_str(&mut self, s: &str) -> Result<(), Error> {
        let rsize = serial_write(s.as_bytes());
        assert_eq!(
            rsize,
            s.as_bytes().len(),
            "Dont' write all bytes through syscall"
        );
        Ok(())
    }
}

#[macro_export]
macro_rules! print {
    ($($arg:tt)*) => ({
        $crate::print(format_args!($($arg)*));
    });
}

#[macro_export]
macro_rules! println {
    ($fmt:expr) => ($crate::print!(concat!($fmt, "\n")));
    ($fmt:expr, $($arg:tt)*) => ($crate::print!(concat!($fmt, "\n"), $($arg)*));
}

#[inline]
pub fn print(args: core::fmt::Arguments) {
    WriteImpl
        .write_fmt(args)
        .expect("can't write string in logging module.");
}
