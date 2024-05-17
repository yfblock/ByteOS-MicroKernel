#![no_std]
#![no_main]
#![feature(exclusive_range_pattern)]

use syscall_consts::{Message, MessageContent, IPC_ANY};
use users::syscall::{ipc_recv, ipc_register};

#[macro_use]
extern crate users;
extern crate alloc;

#[no_mangle]
fn main() {
    let mut message = Message::blank();

    println!("register fs service!");
    ipc_register("fs");

    loop {
        ipc_recv(IPC_ANY, &mut message);
        match message.content {
            // Doing Nothing here.
            MessageContent::PageFaultReply => {}
            _ => println!("unhandled message: {:?}", message.content),
        }
    }
}
