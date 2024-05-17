#![no_std]
#![no_main]
#![feature(exclusive_range_pattern)]

mod virtio_impl;

use core::ptr::NonNull;

use spin::{Lazy, Mutex};
use syscall_consts::{Message, MessageContent, IPC_ANY};
use users::{
    syscall::{ipc_recv, ipc_register, map_paddr},
    PAGE_SIZE,
};
use virtio_drivers::{device::blk::VirtIOBlk, transport::mmio::MmioTransport};
use virtio_impl::HalImpl;

#[macro_use]
extern crate users;
extern crate alloc;

/// VIRTIO 的物理地址，这里特指 virtio-blk
pub const VIRTIO0_PADDR: usize = 0x10008000;

static BLK_DEVICE: Lazy<Mutex<VirtIOBlk<HalImpl, MmioTransport>>> = Lazy::new(|| {
    // TODO: improve security through map attrs
    // 映射物理内存
    let device_vaddr =
        map_paddr(VIRTIO0_PADDR, PAGE_SIZE).expect("can't map virtual address for virtio-blk");

    // 创建 virtio-blk 设备
    Mutex::new(
        VirtIOBlk::<HalImpl, _>::new(
            unsafe {
                MmioTransport::new(NonNull::new(device_vaddr as _).expect("This ptr is zero"))
            }
            .expect("MMio transport is not valid"),
        )
        .expect("This is not a valid virtio block device"),
    )
});

/// 初始化 virtio 驱动
pub fn init_virtio_blk() {
    println!("[blk] init virtio_blk");
    println!(
        "virtio block capacity: {:#x} MB",
        BLK_DEVICE.lock().capacity() / 256
    );
    let buffer = HalImpl::write_buffer();
    BLK_DEVICE
        .lock()
        .read_blocks(0, buffer)
        .expect("can't read block");
    // BLK_DEVICE.lock().write_blocks(0, buffer).expect("can't write block");
}

struct Logger;

impl log::Log for Logger {
    fn enabled(&self, _metadata: &log::Metadata) -> bool {
        true
    }

    fn log(&self, record: &log::Record) {
        static BIG_MUTEX: Mutex<()> = Mutex::new(());
        let _temp_global_lock = BIG_MUTEX.lock();

        let file = record.file();
        let line = record.line();
        println!(
            "[{}] {}:{} {}\n",
            record.level(),
            file.unwrap(),
            line.unwrap(),
            record.args()
        );
    }

    fn flush(&self) {}
}

pub fn init(level: Option<&str>) {
    log::set_logger(&Logger).unwrap();
    log::set_max_level(match level {
        Some("error") => log::LevelFilter::Error,
        Some("warn") => log::LevelFilter::Warn,
        Some("info") => log::LevelFilter::Info,
        Some("debug") => log::LevelFilter::Debug,
        Some("trace") => log::LevelFilter::Trace,
        _ => log::LevelFilter::Off,
    });
}

#[no_mangle]
fn main() {
    init(Some("debug"));
    let mut message = Message::blank();

    init_virtio_blk();

    println!("register blk_device service!");
    ipc_register("blk_device");

    loop {
        ipc_recv(IPC_ANY, &mut message);
        match message.content {
            // MessageContent::BlkWriteReplyMsg()
            // Doing Nothing here.
            MessageContent::PageFaultReply => {}
            _ => println!("unhandled message: {:?}", message.content),
        }
    }
}
