#![no_std]
#![no_main]
#![feature(panic_info_message)]

extern crate alloc;

mod console;
pub mod syscall;

use alloc::string::{String, ToString};
use buddy_system_allocator::LockedHeap;
pub use console::print;
use syscall_consts::SysCallError;

use core::panic::PanicInfo;
use syscall::exit;

/// 页表大小
pub const PAGE_SIZE: usize = 4096;

/// Block 块大小
pub const BLOCK_SIZE: usize = 0x200;

/// 用户程序默认堆大小
const USER_HEAP_SIZE: usize = 0x2000;

/// 堆分配器使用的空间
static mut HEAP_SPACE: [u8; USER_HEAP_SIZE] = [0; USER_HEAP_SIZE];

/// 将 SysCallError 重新导出为 UserError
pub type UserError = SysCallError;

/// 将 a 按照 b 进行 align up
pub fn align_up(a: usize, b: usize) -> usize {
    (a + b - 1) / b * b
}

/// 将 a 按照 b 进行 align down
pub fn align_down(a: usize, b: usize) -> usize {
    a / b * b
}

/// 堆分配器
#[global_allocator]
static HEAP: LockedHeap<32> = LockedHeap::empty();

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
        HEAP.lock()
            .init(HEAP_SPACE.as_ptr() as usize, HEAP_SPACE.len());
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

/// 从 slice 切片中匹配字符串
pub fn get_string_from_slice(buffer: &[u8]) -> String {
    let len = buffer
        .iter()
        .position(|&x| x == b'\0')
        .unwrap_or(buffer.len());
    String::from_utf8_lossy(&buffer[..len]).to_string()
}

#[allow(dead_code)]
pub fn hexdump(data: &[u8], mut start_addr: usize) {
    const PRELAND_WIDTH: usize = 70;
    println!("{:-^1$}", " hexdump ", PRELAND_WIDTH);
    for offset in (0..data.len()).step_by(16) {
        print!("{:08x} ", start_addr);
        start_addr += 0x10;
        for i in 0..16 {
            if offset + i < data.len() {
                print!("{:02x} ", data[offset + i]);
            } else {
                print!("{:02} ", "");
            }
        }

        print!("{:>6}", ' ');

        for i in 0..16 {
            if offset + i < data.len() {
                let c = data[offset + i];
                if c >= 0x20 && c <= 0x7e {
                    print!("{}", c as char);
                } else {
                    print!(".");
                }
            } else {
                print!("{:02} ", "");
            }
        }

        println!("");
    }
    println!("{:-^1$}", " hexdump end ", PRELAND_WIDTH);
}
