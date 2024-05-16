use executor::{tid2task, AsyncTask};
use log::info;
use polyhal::{shutdown, time::Time};
use syscall_consts::{
    IPCFlags, Message, MessageContent, SysCall, SysCallError, FROM_KERNEL, IPC_ANY,
};

use crate::{async_ops::WaitResume, lang_items::puts, task::MicroKernelTask, utils::UserBuffer};

type SysResult = Result<usize, SysCallError>;

impl MicroKernelTask {
    /// 串口输出
    pub fn sys_serial_write(
        &self,
        buf: UserBuffer<u8>,
        buf_len: usize,
    ) -> Result<usize, SysCallError> {
        let bytes = buf.slice_mut_with_len(buf_len);
        puts(bytes);
        Ok(bytes.len())
    }

    /// 退出当前任务
    pub fn sys_task_exit(&self) -> SysResult {
        *self.destoryed.lock() = true;
        Ok(0)
    }

    /// 休眠 ms
    pub fn sys_time(&self, ms: usize) -> SysResult {
        log::trace!("syscall timeout: {}, ms: {}", *self.timeout.lock(), ms);
        *self.timeout.lock() += Time::now().to_nsec() + ms * 1000_000;
        Ok(0)
    }

    /// 获取开机到现在的时间，单位: ms
    pub fn sys_uptime(&self) -> SysResult {
        Ok(Time::now().to_msec())
    }

    /// 关闭计算机
    pub fn sys_shutdown(&self) -> ! {
        shutdown();
    }

    /// 接收 IPC 信息
    pub async fn recv_message(
        &self,
        src: usize,
        message: &mut Message,
        flags: IPCFlags,
    ) -> SysResult {
        // 如果当前 IPC 是 IPC_ANY 且当前的等待通知集不为空，处理通知
        if src == IPC_ANY && !self.notifications.lock().is_empty() {
            message.source = FROM_KERNEL;
            message.content = MessageContent::NotifyField {
                notications: self.notifications.lock().pop_all(),
            };
            return Ok(0);
        }

        // 如果 IPC 含有 NON_BLOCK 标志位，则直接返回
        if flags.contains(IPCFlags::NON_BLOCK) {
            return Err(SysCallError::WouldBlock);
        }

        // 查找目标任务
        let target_tid = self
            .senders
            .lock()
            .iter()
            .find(|tid| src == IPC_ANY || src == **tid)
            .cloned();

        // 恢复目标任务运行
        if let Some(target_tid) = target_tid {
            tid2task(target_tid)
                .expect("can't find sender")
                .downcast_arc::<MicroKernelTask>()
                .map_err(|_| SysCallError::InvalidArg)?
                .resume();
        }

        // 等待来自 src 任务的 IPC
        *self.wait_for.lock() = Some(src);

        // 阻塞当前任务
        self.block();
        WaitResume(self).await;

        // 清空等待状态
        *self.wait_for.lock() = None;

        // 复制消息
        *message = self.message.lock().unwrap();
        Ok(0)
    }

    /// 处理 IPC 请求
    pub async fn sys_ipc(
        &self,
        dst: usize,
        src: usize,
        buffer: UserBuffer<Message>,
        flags: usize,
    ) -> SysResult {
        log::trace!("ipc: {:?}, {:?}, {:?}, {:?}", dst, src, buffer, flags);
        let flags = IPCFlags::from_bits(flags).ok_or(SysCallError::InvalidArg)?;
        info!("ipc dst: {} src: {} flags: {:?}", dst, src, flags);
        if flags.contains(IPCFlags::KERNEL) {
            return Err(SysCallError::InvalidArg);
        }

        // 确保拥有一个有效的 IPC 请求。
        if src != IPC_ANY && tid2task(src).is_none() {
            return Err(SysCallError::InvalidArg);
        }

        // TODO: Write ipc send function
        if flags.contains(IPCFlags::SEND) {}

        // 接收 IPC 消息
        if flags.contains(IPCFlags::RECV) {
            self.recv_message(src, buffer.get_mut(), flags).await?;
        }

        Ok(0)
    }

    /// 处理系统调用
    pub async fn syscall(&self, id: usize, args: [usize; 6]) -> Result<usize, SysCallError> {
        info!("syscall: {:?}", SysCall::try_from(id));
        match SysCall::try_from(id).map_err(|_| SysCallError::InvalidSyscall)? {
            // IPC 请求
            SysCall::IPC => {
                self.sys_ipc(args[0], args[1], args[2].into(), args[3])
                    .await
            }
            SysCall::Notify => todo!(),
            // 串口输出
            SysCall::SerialWrite => self.sys_serial_write(args[0].into(), args[1]),
            SysCall::SerialRead => todo!(),
            SysCall::TaskCreate => todo!(),
            SysCall::TaskDestory => todo!(),
            SysCall::TaskExit => self.sys_task_exit(),
            // 获取当前任务 id
            SysCall::TaskSelf => Ok(self.get_task_id()),
            SysCall::PMAlloc => todo!(),
            SysCall::VMMap => todo!(),
            SysCall::VNUnmap => todo!(),
            SysCall::IrqListen => todo!(),
            SysCall::IrqUnlisten => todo!(),
            // 设置定时器，单位 ms
            SysCall::Time => self.sys_time(args[0]),
            // 获取当前系统时间
            SysCall::UPTime => self.sys_uptime(),
            SysCall::HinaVM => todo!(),
            // 关闭系统
            SysCall::Shutdown => self.sys_shutdown(),
        }
    }
}
