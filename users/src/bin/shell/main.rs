#![no_std]
#![no_main]
#![feature(exclusive_range_pattern)]

use alloc::{string::String, vec::Vec};
use syscall_consts::{Message, MessageContent, IPC_ANY};
use users::syscall::{
    fs_read_dir, get_block_capacity, ipc_call, ipc_recv, serial_read, serial_write, service_lookup,
    sys_time, sys_uptime, task_self,
};

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
                    print!("\n");
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

        match line.as_str() {
            "" => {}
            // Ping-Pong 命令，测试 IPC 和服务
            "ping" => {
                if let Some(task_pong_id) = service_lookup("pong") {
                    message.content = MessageContent::PingMsg(321);
                    println!("Send ping message {} to vm server", 321);
                    ipc_call(task_pong_id, &mut message);
                    println!("Ping message reply {:?}", message.content);
                }
            }
            // 显示所有的 block 设备，目前只有一个
            "disks" => {
                if let Some(blk_dev_tid) = service_lookup("blk_device") {
                    println!(
                        "block device capactiy {} MB",
                        get_block_capacity(blk_dev_tid).unwrap_or(0) / 2048
                    );
                }
            }
            // 列出文件夹下所有的文件
            "ls" => {
                if let Some(fs_tid) = service_lookup("fs") {
                    println!("fs tid is: {}", fs_tid);
                    let files = fs_read_dir(fs_tid, ".");
                    println!("files: {}", files.len());
                    files.iter().for_each(|x| {
                        println!("{:>4} {:<8}", "", x);
                    });
                }
            }
            // 输出帮助信息
            "help" | _ => {
                println!("commands available are below:");
                ["help", "ping", "disks", "ls"].iter().for_each(|x| {
                    println!("{:>10}", x);
                });
            }
        }
    }
}
