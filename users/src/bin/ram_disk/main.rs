#![no_std]
#![no_main]
#![feature(exclusive_range_pattern)]

use core::arch::global_asm;

use syscall_consts::{Message, MessageContent, IPC_ANY};
use users::{
    syscall::{ipc_recv, ipc_register, ipc_reply},
    BLOCK_SIZE,
};

#[macro_use]
extern crate users;
#[macro_use]
extern crate alloc;

global_asm!(
    r#"
    .section .data
    ramdisk_start:
    .incbin "../mount.img"
    ramdisk_end:
"#
);

struct RamDiskImpl;

impl RamDiskImpl {
    /// 读取块
    pub fn read_block(block_id: usize, buffer: &mut [u8]) {
        // 保证缓冲区大小为一个块的大小
        assert_eq!(buffer.len(), BLOCK_SIZE);

        let start = block_id * 0x200;
        let end = start + 0x200;
        buffer.copy_from_slice(&Self::get_ram_disk()[start..end]);
    }

    /// 写入块
    pub fn write_block(block_id: usize, buffer: &[u8]) {
        // 保证缓冲区大小为一个块的大小
        assert_eq!(buffer.len(), BLOCK_SIZE);

        let start = block_id * 0x200;
        let end = start + 0x200;
        Self::get_ram_disk()[start..end].copy_from_slice(&buffer);
    }

    /// 获取 ramdisk 所在的内存区域
    pub fn get_ram_disk() -> &'static mut [u8] {
        extern "C" {
            fn ramdisk_start();
            fn ramdisk_end();
        }
        // 获取 ramdisk 所在的缓冲区
        unsafe {
            core::slice::from_raw_parts_mut(
                ramdisk_start as *mut u8,
                ramdisk_end as usize - ramdisk_start as usize,
            )
        }
    }
}

#[no_mangle]
fn main() {
    let mut message = Message::blank();
    println!("register ramdisk for blk_device service!");
    ipc_register("blk_device");

    loop {
        ipc_recv(IPC_ANY, &mut message);
        match message.content {
            // 获取 块设备容量大小
            MessageContent::GetBlockCapacity => {
                message.content = MessageContent::GetBlockCapacityReplyMsg(
                    RamDiskImpl::get_ram_disk().len() / 0x200,
                );
                ipc_reply(message.source, &mut message);
            }
            // 读取块设备
            MessageContent::ReadBlockMsg { block_index } => {
                let mut buffer = vec![0; BLOCK_SIZE];
                RamDiskImpl::read_block(block_index, &mut buffer);
                message.content = MessageContent::ReadBlockReplyMsg {
                    buffer: buffer.try_into().unwrap(),
                };
                ipc_reply(message.source, &mut message);
            }
            // 写入块设备
            MessageContent::WriteBlockMsg {
                block_index,
                buffer,
            } => {
                RamDiskImpl::write_block(block_index, &buffer);
                message.content = MessageContent::WriteBlockReplyMsg;
                ipc_reply(message.source, &mut message);
            }
            // MessageContent::BlkWriteReplyMsg()
            // Doing Nothing here.
            MessageContent::PageFaultReply => {}
            _ => println!("unhandled message: {:?}", message.content),
        }
    }
}
