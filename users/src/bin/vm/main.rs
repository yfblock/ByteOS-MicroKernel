#![no_std]
#![no_main]
#![feature(concat_idents)]

mod task;

use syscall_consts::{Message, MessageContent::*, IPC_ANY};
use users::syscall::{
    ipc_recv, ipc_reply, shutdown, sys_time, sys_uptime, task_destory, task_self,
};

use crate::task::{spawn_servers, TASK_LIST};

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
    // 启动 servers
    spawn_servers();
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
            // 页错误
            PageFault {
                tid,
                uaddr,
                ip,
                fault,
            } => {
                // println!(
                //     "IPC page fault: {}, {:#x} @ {:#x} reason: {:?}",
                //     tid, uaddr, ip, fault
                // );

                let ret = TASK_LIST
                    .lock()
                    .iter()
                    .find(|x| x.tid == tid)
                    .map(|x| x.handle_page_fault(uaddr, ip, fault))
                    .expect("can't find page fault task in root server");

                if ret.is_err() {
                    println!("task fault: {:?}", ret.err());
                    task_destory(tid);
                    continue;
                }

                message.content = PageFaultReply;
                ipc_reply(tid, &mut message);
            }
            _ => {
                // println!("ipc message: {:#x?}", message);
            }
        }
    }
}
