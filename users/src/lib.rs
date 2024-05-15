#![no_std]
#![no_main]
#![feature(panic_info_message)]

mod console;
mod syscall;

pub use console::print;

use core::panic::PanicInfo;
use syscall::exit;

#[link_section = ".text.entry"]
#[no_mangle]
fn _start() -> ! {
    extern "Rust" {
        fn main();
    }
    unsafe {
        main();
    }
    exit();
}

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    if let Some(location) = info.location() {
        println!("\x1b[1;31m[{}:{}]\x1b[0m", location.file(), location.line(),);
    }
    println!("\x1b[1;31mpanic: '{}'\x1b[0m", info.message().unwrap());
    exit();
}
