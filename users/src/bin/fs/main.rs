#![no_std]
#![no_main]
#![feature(exclusive_range_pattern)]

mod fatfs_shim;

use syscall_consts::{Message, MessageContent, IPC_ANY, NAME_LEN};
use users::syscall::{ipc_recv, ipc_register, ipc_reply, service_lookup};

use crate::fatfs_shim::DiskCursor;

#[macro_use]
extern crate users;
#[macro_use]
extern crate alloc;

#[no_mangle]
fn main() {
    let mut message = Message::blank();

    println!("register fs service!");
    ipc_register("fs");

    // 获取块设备 task id
    let block_device_tid = service_lookup("blk_device").expect("can't find blk_device");

    let cursor: DiskCursor = DiskCursor {
        blk_tid: block_device_tid,
        sector: 0,
        offset: 0,
    };
    // 获取文件系统地址
    let fs = fatfs::FileSystem::new(cursor, fatfs::FsOptions::new()).expect("can't open fatfs");

    println!("[fs] find root dir");
    fs.root_dir().iter().for_each(|x| {
        let file = x.unwrap();
        println!("{:>4} {:<8}  {:#x}", "", file.file_name(), file.len());
    });

    println!("[fs] find block service {}", block_device_tid);
    loop {
        ipc_recv(IPC_ANY, &mut message);
        match message.content {
            // 读取文件夹
            MessageContent::FSReadDirMsg { path, index } => {
                // 定义一个闭包函数用来返回
                let mut reply = |file: Option<&str>| {
                    let mut buffer = [0u8; 2 * NAME_LEN];
                    if let Some(file) = file {
                        let bytes = file.as_bytes();
                        buffer[..bytes.len()].copy_from_slice(bytes);
                    }
                    message.content = MessageContent::FSReadDirReplyMsg {
                        buffer,
                        num: file.map_or_else(|| 0, |_| 1),
                    };
                    ipc_reply(message.source, &mut message);
                };
                // TODO: use path instead of fixed root path
                // let mut path = get_string_from_slice(&path).trim().to_string();
                // let dir = match fs.root_dir().open_dir("/") {
                //     Ok(dir) => dir,
                //     Err(_) => {
                //         reply(None);
                //         continue;
                //     }
                // };
                // 遍历文件夹
                let dir = fs.root_dir();
                match dir.iter().skip(index).next() {
                    Some(file) => reply(Some(&file.unwrap().file_name())),
                    None => reply(None),
                }
            }
            // Doing Nothing here.
            MessageContent::PageFaultReply => {}
            _ => println!("unhandled message: {:?}", message.content),
        }
    }
}
