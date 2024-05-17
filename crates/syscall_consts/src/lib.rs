#![no_std]
#![feature(decl_macro)]

extern crate alloc;

use core::{
    mem::size_of,
    ops::{BitOr, BitOrAssign},
};

use bitflags::bitflags;
use num_enum::{FromPrimitive, IntoPrimitive, TryFromPrimitive};

/// 一个宏，根据参数来表示 bit 位
pub macro bit($x:expr) {
    (1 << ($x))
}

/// 系统调用编号
#[derive(Debug, Clone, Copy, IntoPrimitive, TryFromPrimitive)]
#[repr(usize)]
pub enum SysCall {
    /// IPC
    IPC = 1,
    /// 通知
    Notify = 2,
    /// 串口输出
    SerialWrite = 3,
    /// 串口读取
    SerialRead = 4,
    /// 创建任务
    TaskCreate = 5,
    /// 销毁任务
    TaskDestory = 6,
    /// 退出任务
    TaskExit = 7,
    /// 获取当前任务
    TaskSelf = 8,
    /// 分配物理内存
    PMAlloc = 9,
    /// 映射虚拟内存
    VMMap = 10,
    /// 取消映射虚拟内存
    VMUnmap = 11,
    /// 监听中断
    IrqListen = 12,
    /// 取消监听中断
    IrqUnlisten = 13,
    /// 定时器
    Time = 14,
    /// 获取当前系统时间
    UPTime = 15,
    /// HinaVM
    HinaVM = 16,
    /// 关闭系统
    Shutdown = 17,
}

/// 系统调用的错误
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

/// 消息类型，暂时用不上
/// Rust 可以用 enum 来同时表示消息类型和内容
pub enum MessageType {
    ExceptionMsg = 1,
    PageFaultMsg = 2,
    PageFaultReplyMsg = 3,
    NotifyMsg = 4,
    NotifyIrqMsg = 5,
    NotifyTimerMsg = 6,
    AsyncRecvMsg = 7,
    AsyncRecvReplyMsg = 8,
    PingMsg = 9,
    PingReplyMsg = 10,
    SpawnTaskMsg = 11,
    SpawnTaskReplyMsg = 12,
    DestroyTaskMsg = 13,
    DestroyTaskReplyMsg = 14,
    ServiceLookupMsg = 15,
    ServiceLookupReplyMsg = 16,
    ServiceRegisterMsg = 17,
    ServiceRegisterReplyMsg = 18,
    WatchTasksMsg = 19,
    WatchTasksReplyMsg = 20,
    TaskDestroyedMsg = 21,
    VmMapPhysicalMsg = 22,
    VmMapPhysicalReplyMsg = 23,
    VmAllocPhysicalMsg = 24,
    VmAllocPhysicalReplyMsg = 25,
    BlkReadMsg = 26,
    BlkReadReplyMsg = 27,
    BlkWriteMsg = 28,
    BlkWriteReplyMsg = 29,
    NetOpenMsg = 30,
    NetOpenReplyMsg = 31,
    NetRecvMsg = 32,
    NetSendMsg = 33,
    NetSendReplyMsg = 34,
    FsOpenMsg = 35,
    FsOpenReplyMsg = 36,
    FsCloseMsg = 37,
    FsCloseReplyMsg = 38,
    FsReadMsg = 39,
    FsReadReplyMsg = 40,
    FsWriteMsg = 41,
    FsWriteReplyMsg = 42,
    FsReaddirMsg = 43,
    FsReaddirReplyMsg = 44,
    FsMkfileMsg = 45,
    FsMkfileReplyMsg = 46,
    FsMkdirMsg = 47,
    FsMkdirReplyMsg = 48,
    FsDeleteMsg = 49,
    FsDeleteReplyMsg = 50,
    TcpipConnectMsg = 51,
    TcpipConnectReplyMsg = 52,
    TcpipCloseMsg = 53,
    TcpipCloseReplyMsg = 54,
    TcpipWriteMsg = 55,
    TcpipWriteReplyMsg = 56,
    TcpipReadMsg = 57,
    TcpipReadReplyMsg = 58,
    TcpipDnsResolveMsg = 59,
    TcpipDnsResolveReplyMsg = 60,
    TcpipDataMsg = 61,
    TcpipClosedmsg = 62,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Notify(usize);

impl Notify {
    /// 创建空的 Notifications
    pub const fn new() -> Self {
        Notify(0)
    }

    /// 弹出所有的通知，返回的结构体还是自身
    pub fn pop_all(&mut self) -> Notify {
        let ret = Notify(self.0);
        self.0 = 0;
        ret
    }

    /// 弹出一条通知，返回结构为 [NotifyEnum]
    pub fn pop(&mut self) -> Option<NotifyEnum> {
        for i in 0..size_of::<usize>() * 8 {
            if self.0 & bit!(i) != 0 {
                self.0 &= !bit!(i);
                match i {
                    0 => return Some(NotifyEnum::TIMER),
                    1 => return Some(NotifyEnum::IRQ),
                    2 => return Some(NotifyEnum::ABORTED),
                    _ => return Some(NotifyEnum::ASYNC(i as u8 - 3)),
                }
            }
        }
        None
    }

    /// 弹出指定 Notify
    pub fn pop_specify(&mut self, notification: NotifyEnum) -> Option<NotifyEnum> {
        // 根据 Notification 获取 index
        let index = match notification {
            NotifyEnum::TIMER => 0,
            NotifyEnum::IRQ => 1,
            NotifyEnum::ABORTED => 2,
            NotifyEnum::ASYNC(tid) => 3 + tid as usize,
        };
        match self.0 & bit!(index) != 0 {
            // 含有特定的 Notification
            true => {
                self.0 &= !bit!(index);
                Some(notification)
            }
            false => None,
        }
    }

    /// 判断是否含有 Notifications
    pub fn is_empty(&self) -> bool {
        self.0 == 0
    }
}

/// 重载 | 运算符
/// 可以让多个通知进行 notify1 | notify2 合成一个 Notify 集合
impl BitOr for Notify {
    type Output = Self;

    fn bitor(self, rhs: Self) -> Self::Output {
        Notify(self.0 | rhs.0)
    }
}

/// 重载 |= 运算符
/// 可以方便合成多个 Notify
impl BitOrAssign for Notify {
    fn bitor_assign(&mut self, rhs: Self) {
        self.0 |= rhs.0;
    }
}

/// 这个结构来表示 [Notify] 的其中一个 notification
#[derive(Debug, Clone, Copy)]
pub enum NotifyEnum {
    /// 时钟通知
    TIMER,
    /// 中断通知
    IRQ,
    ABORTED,
    ASYNC(u8),
}

impl From<NotifyEnum> for Notify {
    fn from(value: NotifyEnum) -> Self {
        match value {
            NotifyEnum::TIMER => Notify(bit!(0)),
            NotifyEnum::IRQ => Notify(bit!(1)),
            NotifyEnum::ABORTED => Notify(bit!(2)),
            NotifyEnum::ASYNC(tid) => Notify(bit!(3 + tid as usize)),
        }
    }
}

/// 一般用在 [SysCall::IPC] 的参数，表示接收任一 user app 发送的 IPC 消息
pub const IPC_ANY: usize = 0;

/// 一般用在 [Message::source] 表示从内核发送的任务
pub const FROM_KERNEL: usize = usize::MAX;

/// 指 ROOT SERVER
pub const VM_SERVER: usize = 1;

/// 存储 Service Name 的字符串长度
pub const NAME_LEN: usize = 64;

/// 消息内容，这是一个 Rust 的 enum 结构
/// 后续可以在这个里面添加消息结构以增加消息的类型。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MessageContent {
    /// 页错误消息
    PageFault {
        tid: usize,
        uaddr: usize,
        ip: usize,
        fault: PageFaultReason,
    },
    /// 页错误回复消息
    PageFaultReply,
    /// 通知消息
    NotifyField {
        notications: Notify,
    },
    /// Ping
    PingMsg(usize),
    /// pong
    PingReplyMsg(usize),
    /// 中断
    NotifyIRQ,
    /// 定时器
    NotifyTimer,
    /// 服务注册消息
    ServiceRegisterMsg {
        name_buffer: [u8; NAME_LEN],
    },
    /// 服务注册消息回复
    ServiceRegisterReplyMsg,
    /// 服务注册消息
    ServiceLookupMsg {
        name_buffer: [u8; NAME_LEN],
    },
    /// 服务注册消息回复，携带任务 id
    ServiceLookupReplyMsg(usize),
    None,
}

/// 消息结构
/// `src` 是从哪个任务传递过来的消息
/// `content` 是消息的内容，这是一个 enum 结构
/// 由于 kernel 和 user app 都是 Rust，所以可以采用 Rust 的 enum
#[derive(Debug, Clone)]
pub struct Message {
    pub source: usize,
    pub content: MessageContent,
}

impl Message {
    pub fn blank() -> Self {
        Message {
            source: 0,
            content: MessageContent::None,
        }
    }
}

bitflags! {
    /// IPC 标志位
    #[derive(Debug, Clone, Copy)]
    pub struct IPCFlags: usize {
        const SEND      =  bit!(16);
        const RECV      =  bit!(17);
        const NON_BLOCK =  bit!(18);
        const KERNEL    =  bit!(19);
        const CALL      = Self::SEND.bits() | Self::RECV.bits();
    }

    /// 页错误的原因
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct PageFaultReason: usize {
        const READ      = bit!(0);
        const WRITE     = bit!(1);
        const EXEC      = bit!(2);
        const USER      = bit!(3);
        const PRESENT   = bit!(4);
    }

    /// 申请内存 Flags
    #[derive(Debug, Clone, Copy)]
    pub struct PMAllocFlags: usize {
        const UNINITIALIZED = bit!(0);
        const ZEROD         = bit!(1);
        const ALIGNED       = bit!(2);
    }
}

/// 异常类型
#[derive(Debug, Clone, Copy)]
pub enum ExceptionType {
    GraceExit,
    InvalidAddr,
    InvalidPagerReply,
    IllegalException,
}
