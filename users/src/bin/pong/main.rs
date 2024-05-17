#![no_std]
#![no_main]
#![feature(exclusive_range_pattern)]

use syscall_consts::{Message, MessageContent, IPC_ANY};
use users::syscall::{ipc_recv, ipc_register, ipc_reply, task_self};

#[macro_use]
extern crate users;
extern crate alloc;

#[no_mangle]
fn main() {
    let mut message = Message::blank();

    println!("register ping service!");
    ipc_register("pong");

    loop {
        ipc_recv(IPC_ANY, &mut message);
        match message.content {
            MessageContent::PingMsg(value) => {
                println!("task {} received ping {}", task_self(), value);
                message.content = MessageContent::PingReplyMsg(42);
                ipc_reply(message.source, &mut message);
            }
            // Doing Nothing here.
            MessageContent::PageFaultReply => {}
            _ => println!("unhandled message: {:?}", message.content),
        }
    }
}
