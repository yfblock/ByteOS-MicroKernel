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

#[repr(isize)]
#[derive(Debug, Clone, Copy, IntoPrimitive, FromPrimitive)]
pub enum SysCallError {
    NoMemory = -1,        // 内存不足
    NoResources = -2,     // 没有足够的资源
    AlreadyExists = -3,   // 已经存在
    AlreadyUsed = -4,     // 已使用
    AlreadyDone = -5,     // 已经完成
    StillUsed = -6,       // 仍在使用中
    NotFound = -7,        // 找不到
    NotAllowed = -8,      // 未授权
    NotSupported = -9,    // 不支持
    Unexpected = -10,     // 意外的输入值/情况
    InvalidArg = -11,     // 无效参数/输入值
    InvalidTask = -12,    // 无效的任务ID
    InvalidSyscall = -13, // 无效的系统调用号
    InvalidPaddr = -14,   // 无效的物理地址
    InvalidUaddr = -15,   // 无效的用户空间地址
    TooManyTasks = -16,   // 任务过多
    TooLarge = -17,       // 太大
    TooSmall = -18,       // 太小
    WouldBlock = -19,     // 被中断，因为它会阻塞
    TryAgain = -20,       // 暂时失败：重试可能会成功
    Aborted = -21,        // 中断
    Empty = -22,          // 是空的
    NotEmpty = -23,       // 不是空的
    DeadLock = -24,       // 发生死锁
    NotAFile = -25,       // 不是一个文件
    NotADir = -26,        // 不是目录
    EOF = -27,            // 文件数据结束
    END = -28,            // 必须是最后一个错误码
    #[default]
    Others = -29,
}
