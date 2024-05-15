use core::fmt::Write;
use core::panic::PanicInfo;
use log::{Level, LevelFilter, Log};
use polyhal::{addr::PhysPage, debug::DebugConsole, hart_id, PageAlloc};
use spin::Mutex;

use crate::frame;

struct Logger;

impl Log for Logger {
    fn enabled(&self, _metadata: &log::Metadata) -> bool {
        true
    }

    fn log(&self, record: &log::Record) {
        static BIG_MUTEX: Mutex<()> = Mutex::new(());
        let _temp_global_lock = BIG_MUTEX.lock();

        let color_code = match record.level() {
            Level::Error => 31u8, // Red
            Level::Warn => 93,    // BrightYellow
            Level::Info => 34,    // Blue
            Level::Debug => 32,   // Green
            Level::Trace => 90,   // BrightBlack
        };
        let file = record.file();
        let line = record.line();
        write!(
            DebugConsole,
            "\u{1B}[{}m\
                [{}] {}:{} {}\
                \u{1B}[0m\n",
            color_code,
            record.level(),
            file.unwrap(),
            line.unwrap(),
            record.args()
        )
        .expect("can't write color string in logging module.");
    }

    fn flush(&self) {}
}

pub fn init(level: Option<&str>) {
    log::set_logger(&Logger).unwrap();
    log::set_max_level(match level {
        Some("error") => LevelFilter::Error,
        Some("warn") => LevelFilter::Warn,
        Some("info") => LevelFilter::Info,
        Some("debug") => LevelFilter::Debug,
        Some("trace") => LevelFilter::Trace,
        _ => LevelFilter::Off,
    });
    log::info!("logging module initialized");
}

#[macro_export]
macro_rules! print {
    ($($arg:tt)*) => ({
        $crate::lang_items::print(format_args!($($arg)*));
    });
}

#[macro_export]
macro_rules! println {
    ($fmt:expr) => ($crate::print!(concat!($fmt, "\n")));
    ($fmt:expr, $($arg:tt)*) => ($crate::print!(concat!($fmt, "\n"), $($arg)*));
}

#[inline]
pub fn print(args: core::fmt::Arguments) {
    DebugConsole
        .write_fmt(args)
        .expect("can't write string in logging module.");
}

pub(crate) struct PageAlloImpl;

impl PageAlloc for PageAlloImpl {
    fn alloc(&self) -> PhysPage {
        frame::frame_alloc_persist()
    }

    fn dealloc(&self, ppn: PhysPage) {
        frame::frame_dealloc(ppn)
    }
}

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    if let Some(location) = info.location() {
        println!(
            "\x1b[1;31m[Core {}] [{}:{}]\x1b[0m",
            hart_id(),
            location.file(),
            location.line(),
        );
    }
    println!(
        "\x1b[1;31m[Core {}] panic: '{}'\x1b[0m",
        hart_id(),
        info.message().unwrap()
    );
    // backtrace();
    println!("!TEST FINISH!");
    polyhal::shutdown();
}
