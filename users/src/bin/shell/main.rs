#![no_std]
#![no_main]
#![feature(exclusive_range_pattern)]

use alloc::{string::String, vec::Vec};
use syscall_consts::{Message, IPC_ANY};
use users::syscall::{ipc_recv, serial_read, serial_write, sys_time, sys_uptime, task_self};

#[macro_use]
extern crate users;
extern crate alloc;

// 字符集合
const LF: u8 = b'\n';
const CR: u8 = b'\r';
const DL: u8 = b'\x7f';
const BS: u8 = b'\x08';
const SPACE: u8 = b' ';

/// 读取一行数据
fn read_line() -> String {
    let mut tmp = [0u8; 32];
    let mut buffer = Vec::new();
    loop {
        let len = serial_read(&mut tmp);
        if len == 0 {
            continue;
        }

        assert_eq!(len, 1, "len should be 1, tip me if not");

        for i in 0..len {
            match tmp[i] as u8 {
                // 如果是换行符
                CR | LF => {
                    // 当前已经接收的字符数量为 0
                    if buffer.len() != 0 {
                        print!("\n");
                    }
                    return String::from_utf8(buffer).expect("This is not a valid utf8 string");
                }
                // 如果是退格符
                BS | DL => {
                    if buffer.len() > 0 {
                        buffer.pop();
                        serial_write(&[BS, SPACE, BS]);
                    }
                }
                // 特殊字符
                0..30 => {}
                // 其他字符
                _ => {
                    buffer.push(tmp[i] as u8);
                    serial_write(&tmp[i..i + 1]);
                }
            }
        }
    }
}

#[no_mangle]
fn main() {
    let mut message = Message::blank();
    println!("Hello shell!");
    println!("Shell server id: {}", task_self());
    // 输出系统时间
    println!("UPTIME: {}", sys_uptime());

    // 等待 100ms 其他任务启动完毕，否则 log 可能会混乱
    sys_time(100);
    ipc_recv(IPC_ANY, &mut message);

    loop {
        print!("\x1b[1mshell> \x1b[0m");

        // 读取一行输入
        let line = read_line();

        println!("{}", line);
    }
}
