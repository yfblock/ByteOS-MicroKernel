#![no_std]
#![no_main]
#![feature(concat_idents)]

mod task;

use alloc::string::{String, ToString};
use syscall_consts::{
    Message,
    MessageContent::{self, *},
    IPC_ANY,
};
use users::syscall::{ipc_recv, ipc_reply, sys_time, sys_uptime, task_destory, task_self};

use crate::task::{register_service, spawn_servers, SERVICE_LIST, TASK_LIST};

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
                // shutdown();
            }
            // 服务注册消息
            ServiceRegisterMsg { name_buffer } => {
                // 确保注册服务的来源是已经在任务列表中
                assert!(TASK_LIST
                    .lock()
                    .iter()
                    .find(|x| x.tid == message.source)
                    .is_some());
                // 获取需要注册的服务名称
                let name = String::from_utf8_lossy(&name_buffer).to_string();

                // 注册一个服务
                register_service(message.source, name);

                // 回复消息
                message.content = MessageContent::ServiceRegisterReplyMsg;
                ipc_reply(message.source, &mut message);
            }
            // 服务查找消息
            ServiceLookupMsg { name_buffer } => {
                // 获取需要搜索的服务名称
                let name = String::from_utf8_lossy(&name_buffer).to_string();
                // 在服务列表中查找服务
                let service = SERVICE_LIST.lock().iter().find(|x| x.name == name).cloned();
                // 如果服务已经注册了，直接处理
                // 如果未注册，那么等待注册后唤醒
                if let Some(service) = service {
                    message.content = MessageContent::ServiceLookupReplyMsg(service.task_id);
                    ipc_reply(message.source, &mut message);
                } else {
                    TASK_LIST
                        .lock()
                        .iter_mut()
                        .find(|x| x.tid == message.source)
                        .map(|x| {
                            x.waiting_for = name;
                        });
                }
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
