#![no_std]
#![no_main]
#![feature(panic_info_message)]

mod console;
pub mod syscall;

use buddy_system_allocator::LockedHeap;
pub use console::print;

use core::panic::PanicInfo;
use syscall::exit;

const USER_HEAP_SIZE: usize = 0x2000;

static mut HEAP_SPACE: [u8; USER_HEAP_SIZE] = [0; USER_HEAP_SIZE];

#[global_allocator]
static HEAP: LockedHeap::<32> = LockedHeap::empty();

#[link_section = ".text.entry"]
#[no_mangle]
fn _start() -> ! {
    extern "Rust" {
        fn main();
    }

    unsafe {
        HEAP.lock().init(HEAP_SPACE.as_ptr() as usize, HEAP_SPACE.len());
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
