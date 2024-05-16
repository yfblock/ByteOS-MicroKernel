#![no_std]
#![no_main]

use users::syscall::{exit, sys_uptime, task_self};

#[macro_use]
extern crate users;
extern crate alloc;

#[no_mangle]
fn main() {
    println!("Hello World!");
    println!("Shell server id: {}", task_self());
    // 输出系统时间
    println!("UPTIME: {}", sys_uptime());
    // 退出当前任务
    exit()
}
