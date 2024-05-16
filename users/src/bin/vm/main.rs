#![no_std]
#![no_main]

mod task;

use syscall_consts::{Message, MessageContent::*, IPC_ANY};
use users::syscall::{ipc_recv, shutdown, sys_time, sys_uptime, task_self};

#[macro_use]
extern crate users;
extern crate alloc;

#[no_mangle]
fn main() {
    println!("Hello World!");
    println!("Root server id: {}", task_self());
    // 设置定时器
    sys_time(5000);
    // 输出系统时间
    println!("UPTIME: {}", sys_uptime());
    loop {
        let mut message = Message::blank();
        // 等待并接收 IPC 消息
        ipc_recv(IPC_ANY, &mut message);
        match message.content {
            // 时钟消息
            NotifyTimer => {
                println!("Notify Timer");
                println!("UPTIME: {}", sys_uptime());
                shutdown();
            }
            _ => {},
        }
        
    }
}
