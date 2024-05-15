#![no_std]

use num_enum::{FromPrimitive, IntoPrimitive};

#[derive(Debug, Clone, Copy, IntoPrimitive, FromPrimitive)]
#[repr(usize)]
pub enum SysCall {
    IPC = 1,
    Notify = 2,
    SerialWrite = 3,
    SerialRead = 4,
    TaskCreate = 5,
    TaskDestory = 6,
    TaskExit = 7,
    TaskSelf = 8,
    PMAlloc = 9,
    VMMap = 10,
    VNUnmap = 11,
    IrqListen = 12,
    IrqUnlisten = 13,
    Time = 14,
    UPTime = 15,
    HinaVM = 16,
    Shutdown = 17,
    #[default]
    Unknown,
}
