use core::{arch::asm, panic};

use spin::Mutex;
use syscall_consts::{IPCFlags, Message, MessageContent, Notify, NotifyEnum::{self, IRQ, TIMER}, SysCall, IPC_ANY};

/// riscv64 发送 syscall
#[cfg(target_arch = "riscv64")]
#[inline]
fn syscall(id: usize, args: [usize; 4]) -> isize {
    let mut ret: isize;
    unsafe {
        asm!(
            "ecall",
            inlateout("x10") args[0] => ret,
            in("x11") args[1],
            in("x12") args[2],
            in("x13") args[3],
            in("x17") id
        );
    }
    ret
}

/// aarch64 发送 syscall
#[cfg(target_arch = "aarch64")]
#[inline]
fn syscall(id: usize, args: [usize; 3]) -> isize {
    let mut ret: isize;
    unsafe {
        asm!(
            "svc #0",
            inlateout("x0") args[0] => ret,
            in("x1") args[1],
            in("x2") args[2],
            in("x8") id
        );
    }
    ret
}

/// x86_64 发送 syscall
#[cfg(target_arch = "x86_64")]
#[inline]
fn syscall(id: usize, args: [usize; 3]) -> isize {
    let mut ret: isize;
    unsafe {
        asm!(
            "
                push r11
                push rcx
                syscall
                pop  rcx
                pop  r11
            ",
            in("rdi") args[0],
            in("rsi") args[1],
            in("rdx") args[2],
            inlateout("rax") id => ret
        );
    }
    ret
}

/// loongarch64 发送 syscall
#[cfg(target_arch = "loongarch64")]
#[inline]
fn syscall(id: usize, args: [usize; 3]) -> isize {
    let mut ret: isize;
    unsafe {
        asm!(
            "syscall 0",
            inlateout("$r4") args[0] => ret,
            in("$r5") args[1],
            in("$r6") args[2],
            in("$r11") id
        );
    }
    ret
}

/// 等待处理的通知集合，这其实是一个 bitset
static PENDING_NOTIFICATIONS: Mutex<Notify> = Mutex::new(Notify::new());

/// 将通知转换为消息
pub fn recv_notification_as_message(message: &mut Message) -> isize {
    assert!(!PENDING_NOTIFICATIONS.lock().is_empty());
    match PENDING_NOTIFICATIONS.lock().pop().unwrap() {
        TIMER => {
            message.content = MessageContent::NotifyTimer;
            0
        },
        IRQ => {
            message.content = MessageContent::NotifyIRQ;
            0
        },
        NotifyEnum::ASYNC(tid) => todo!("tid: {tid} implment async task notification"),
        unexpected => panic!("unhandled notification: {:?}", unexpected)
    }
}

/// 接受 any 的 message
pub fn ip_recv_any(message: &mut Message) -> isize {
    loop {
        // 如果有收到通知，则将通知转换为消息并返回。
        if !PENDING_NOTIFICATIONS.lock().is_empty() {
            return recv_notification_as_message(message);
        }
        // 发送 IPC 请求，阻塞直到有消息返回
        let ret = sys_ipc(0, IPC_ANY, message, IPCFlags::RECV);
        if ret != 0 {
            return ret;
        }

        // 匹配消息内容
        match message.content {
            // 如果是通知消息，则可能是消息集合
            // 写入 PENDING_NOTIFICATIONS，然后单个处理
            MessageContent::NotifyField { notications } => {
                // TODO: Check src is from kernel, if not print warning and ignore it.
                *PENDING_NOTIFICATIONS.lock() |= notications;
                return recv_notification_as_message(message);
            },
            _ => {
                if ret < 0 {

                }
            }
        }
    }
}

// 接受 IPC 消息
pub fn ipc_recv(src: usize, message: &mut Message) -> isize {
    if src == IPC_ANY {
        return ip_recv_any(message);
    }

    // 发送 IPC 请求，阻塞直到有消息返回
    sys_ipc(0, src, message, IPCFlags::RECV)
}

/// 发送或接收 IPC
#[inline]
pub fn sys_ipc(dst: usize, src: usize, message: &mut Message, flags: IPCFlags) -> isize {
    syscall(SysCall::IPC.into(), [dst, src, message as *mut _ as usize, flags.bits()])
}

/// 串口输出
#[inline]
pub fn serial_write(buf: &[u8]) -> usize {
    syscall(SysCall::SerialWrite.into(), [buf.as_ptr() as usize, buf.len(), 0, 0]) as _
}

/// 设置一个定时器, 时间到了内核会发送 Notification (单位: ms)
#[inline]
pub fn sys_time(ms: usize) -> usize {
    syscall(SysCall::Time.into(), [ms, 0, 0, 0]) as _
}

/// 获取从开机到现在多长时间 (单位: ms)
#[inline]
pub fn sys_uptime() -> usize {
    syscall(SysCall::UPTime.into(), [0, 0, 0, 0]) as _
}

/// 退出当前任务
#[inline]
pub fn exit() -> ! {
    syscall(SysCall::TaskExit.into(), [0, 0, 0, 0]);
    unreachable!("This task should already exited.")
}

/// 关机
#[inline]
pub fn shutdown() -> ! {
    syscall(SysCall::Shutdown.into(), Default::default());
    unreachable!("This computor should shutdown.")
}

/// 获取当前的任务 id
pub fn task_self() -> usize {
    static TASK_SELF: Mutex<usize> = Mutex::new(0);
    if *TASK_SELF.lock() > 0 {
        return *TASK_SELF.lock();
    }

    let tid = syscall(SysCall::TaskSelf.into(), Default::default()) as usize;
    *TASK_SELF.lock() = tid;
    tid
}
