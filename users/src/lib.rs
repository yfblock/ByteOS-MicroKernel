#![no_std]
#![no_main]
#![feature(panic_info_message)]

mod console;
pub mod syscall;

use buddy_system_allocator::LockedHeap;
pub use console::print;

use core::panic::PanicInfo;
use syscall::exit;

/// 用户程序默认堆大小
const USER_HEAP_SIZE: usize = 0x2000;

/// 堆分配器使用的空间
static mut HEAP_SPACE: [u8; USER_HEAP_SIZE] = [0; USER_HEAP_SIZE];

/// 堆分配器
#[global_allocator]
static HEAP: LockedHeap::<32> = LockedHeap::empty();

/// 程序真正的入口，会在这里进行初始化
#[link_section = ".text.entry"]
#[no_mangle]
fn _start() -> ! {
    extern "Rust" {
        fn main();
        fn _sbss();
        fn _ebss();
    }

    unsafe {
        // Clear BSS
        core::slice::from_raw_parts_mut(_sbss as *mut u8, _ebss as usize - _sbss as usize).fill(0);
        // Init heap allocator
        HEAP.lock().init(HEAP_SPACE.as_ptr() as usize, HEAP_SPACE.len());
        // Call main function
        main();
    }
    exit();
}

/// Panic 处理程序
#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    // 如果 panic 信息中含有位置信息，则输出位置信息
    if let Some(location) = info.location() {
        println!("\x1b[1;31m[{}:{}]\x1b[0m", location.file(), location.line(),);
    }
    // 输出 panic 信息
    println!("\x1b[1;31mpanic: '{}'\x1b[0m", info.message().unwrap());
    // 退出当前任务
    exit();
}
