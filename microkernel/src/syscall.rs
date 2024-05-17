use executor::{tid2task, yield_now, AsyncTask};
use log::info;
use polyhal::{
    addr::{PhysPage, VirtAddr, VirtPage},
    debug::DebugConsole,
    pagetable::PageTable,
    shutdown,
    time::Time,
};

use syscall_consts::{
    IPCFlags, Message, MessageContent, NotifyEnum, PMAllocFlags, SysCall, SysCallError,
    FROM_KERNEL, IPC_ANY,
};

use crate::{
    async_ops::WaitResume,
    lang_items::puts,
    task::{MicroKernelTask, TaskState},
    utils::UserBuffer,
};

type SysResult = Result<usize, SysCallError>;

impl MicroKernelTask {
    /// 串口输出
    pub async fn sys_serial_write(
        &self,
        buf: UserBuffer<u8>,
        buf_len: usize,
    ) -> Result<usize, SysCallError> {
        let bytes = buf.slice_mut_with_len(buf_len, self).await;
        puts(bytes);
        Ok(bytes.len())
    }

    /// 串口输入，返回值为读取的字符数
    pub async fn sys_serial_read(
        &self,
        buf: UserBuffer<u8>,
        buf_len: usize,
    ) -> Result<usize, SysCallError> {
        let bytes = buf.slice_mut_with_len(buf_len, self).await;
        assert!(bytes.len() > 0, "buffer is not a valid buffer");
        // 读取串口数据 直到有输出
        loop {
            if let Some(c) = DebugConsole::getchar() {
                bytes[0] = c;
                break;
            }
            yield_now().await;
        }
        Ok(1)
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

    /// 发送 IPC 信息
    pub async fn send_message(
        &self,
        dst: usize,
        message: &mut Message,
        flags: IPCFlags,
    ) -> SysResult {
        log::trace!("task {} send {:?} to {}", self.tid, message, dst);
        // 不能给自己发送 IPC
        if dst == self.tid {
            return Err(SysCallError::InvalidArg);
        }

        // 获取收信任务
        let dst = tid2task(dst)
            .ok_or(SysCallError::InvalidArg)?
            .downcast_arc::<MicroKernelTask>()
            .map_err(|_| SysCallError::InvalidArg)?;

        // 判断目的任务是否正在准备接受信息
        let ready = {
            let dst_state = dst.state.lock();
            let dst_wait_for = dst.wait_for.lock();
            *dst_state == TaskState::Blocked
                && (*dst_wait_for == Some(IPC_ANY) || *dst_wait_for == Some(self.tid))
        };

        // 如果目的任务并没有处于等待状态
        if !ready {
            // 如果 IPC 含有 NON_BLOCK 标志位，则直接返回
            if flags.contains(IPCFlags::NON_BLOCK) {
                return Err(SysCallError::WouldBlock);
            }

            // 如果目的任务也在等待给当前任务发送消息，会发生死锁
            if self
                .senders
                .lock()
                .iter()
                .find(|x| **x == dst.tid)
                .is_some()
            {
                log::error!("deadlock");
                return Err(SysCallError::DeadLock);
            }

            // 将当前任务添加到目的任务的等待列表
            dst.senders.lock().push(self.tid);

            // 阻塞当前任务
            self.block();

            // 等待当前任务恢复
            WaitResume(self).await;

            // 如果目标任务已经完成
            if self
                .notifications
                .lock()
                .pop_specify(NotifyEnum::ABORTED)
                .is_some()
            {
                return Err(SysCallError::Aborted);
            }
        }
        // 设置 message 信息
        let source = if flags.contains(IPCFlags::KERNEL) {
            FROM_KERNEL
        } else {
            self.tid
        };
        *dst.message.lock() = Some(Message {
            source,
            content: message.content.clone(),
        });
        // 恢复 dst 任务运行
        dst.resume();
        Ok(0)
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
            // 删除 senders 中的目标任务
            self.senders.lock().retain(|x| *x != target_tid);
        }

        // TIPS: 如果是唤醒了 target_tid 的任务，那么只等待它, 因为那里已经在阻塞了
        // 否则可能出现消息丢失
        *self.wait_for.lock() = Some(target_tid.unwrap_or(src));

        // 阻塞当前任务
        self.block();
        WaitResume(self).await;

        // 清空等待状态
        *self.wait_for.lock() = None;

        // 复制消息
        *message = self.message.lock().clone().unwrap();
        Ok(0)
    }

    /// 进行 IPC 通信
    pub async fn ipc(
        &self,
        dst: usize,
        src: usize,
        message: &mut Message,
        flags: IPCFlags,
    ) -> SysResult {
        // 发送 IPC 消息
        if flags.contains(IPCFlags::SEND) {
            self.send_message(dst, message, flags).await?;
        }

        // 接收 IPC 消息
        if flags.contains(IPCFlags::RECV) {
            self.recv_message(src, message, flags).await?;
        }

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
        info!(
            "[task {}] ipc dst: {} src: {} flags: {:?}",
            self.tid, dst, src, flags
        );
        if flags.contains(IPCFlags::KERNEL) {
            return Err(SysCallError::InvalidArg);
        }

        // 确保拥有一个有效的 IPC 请求。
        if src != IPC_ANY && tid2task(src).is_none() {
            return Err(SysCallError::InvalidArg);
        }

        self.ipc(dst, src, buffer.get_mut(self).await, flags).await
    }

    /// 创建新的任务
    pub async fn sys_task_create(
        &self,
        name_buf: UserBuffer<u8>,
        entry_point: usize,
        pager: usize,
    ) -> SysResult {
        let name = name_buf.get_str(self).await.map_err(|err| {
            log::error!("{:#x?}", err);
            SysCallError::InvalidArg
        })?;

        let pager = tid2task(pager)
            .map(|x| x.downcast_arc::<MicroKernelTask>().ok())
            .flatten();

        Ok(Self::new(&name, entry_point, pager))
    }

    /// 申请物理内存
    pub fn sys_pm_alloc(&self, dst: usize, size: usize, flags: usize) -> SysResult {
        let mut flags = PMAllocFlags::from_bits(flags).ok_or(SysCallError::InvalidArg)?;

        flags |= PMAllocFlags::ZEROD;
        // 如果需要申请页表的任务就是当前任务
        // 直接处理
        if dst == self.tid {
            return Ok(self.alloc_memory(size, flags));
        }

        // 获取申请内存的任务
        let dst = tid2task(dst)
            .ok_or(SysCallError::InvalidTask)?
            .downcast_arc::<MicroKernelTask>()
            .map_err(|_| SysCallError::InvalidTask)?;

        // 如果 dst 任务和当前任务不存在联系
        if dst.pager.as_ref().ok_or(SysCallError::InvalidTask)?.tid != self.tid {
            return Err(SysCallError::InvalidTask);
        }

        // 为 dst 任务申请页表
        Ok(dst.alloc_memory(size, flags))
    }

    /// 映射虚拟内存
    /// TODO: use flags to control page privilege
    pub fn sys_vm_map(&self, dst: usize, uaddr: usize, paddr: usize, _flags: usize) -> SysResult {
        // 如果需要申请页表的任务就是当前任务
        // 直接处理
        let vpn = VirtPage::from_addr(uaddr);
        let ppn = PhysPage::from_addr(paddr);
        if dst == self.tid {
            // 映射内存
            self.map_page(vpn, ppn);
            return Ok(0);
        }

        // 获取申请内存的任务
        let dst = tid2task(dst)
            .ok_or(SysCallError::InvalidTask)?
            .downcast_arc::<MicroKernelTask>()
            .map_err(|_| SysCallError::InvalidTask)?;

        // 如果 dst 任务和当前任务不存在联系
        if dst.pager.as_ref().ok_or(SysCallError::InvalidTask)?.tid != self.tid {
            return Err(SysCallError::InvalidTask);
        }

        dst.map_page(vpn, ppn);

        Ok(0)
    }

    /// 取消映射虚拟内存
    pub fn sys_vm_unmap(&self, dst: usize, uaddr: usize) -> SysResult {
        // 如果需要申请页表的任务就是当前任务
        // 直接处理
        let vpn = VirtPage::from_addr(uaddr);
        if dst == self.tid {
            // 映射内存
            self.page_table().unmap_page(vpn);
            return Ok(0);
        }

        // 获取申请内存的任务
        let dst = tid2task(dst)
            .ok_or(SysCallError::InvalidTask)?
            .downcast_arc::<MicroKernelTask>()
            .map_err(|_| SysCallError::InvalidTask)?;

        // 如果 dst 任务和当前任务不存在联系
        if dst.pager.as_ref().ok_or(SysCallError::InvalidTask)?.tid != self.tid {
            return Err(SysCallError::InvalidTask);
        }

        dst.page_table().unmap_page(vpn);
        Ok(0)
    }

    /// 翻译虚拟地址
    pub fn sys_trans_paddr(&self, uaddr: usize) -> SysResult {
        Ok(PageTable::current()
            .translate(VirtAddr::new(uaddr))
            .ok_or(SysCallError::InvalidUaddr)?
            .0
            .addr())
    }

    /// 处理系统调用
    pub async fn syscall(&self, id: usize, args: [usize; 6]) -> Result<usize, SysCallError> {
        info!("task: {} syscall: {:?}", self.tid, SysCall::try_from(id));
        match SysCall::try_from(id).map_err(|_| SysCallError::InvalidSyscall)? {
            // IPC 请求
            SysCall::IPC => {
                self.sys_ipc(args[0], args[1], args[2].into(), args[3])
                    .await
            }
            SysCall::Notify => todo!(),
            // 串口输出
            SysCall::SerialWrite => self.sys_serial_write(args[0].into(), args[1]).await,
            // 串口输入
            SysCall::SerialRead => self.sys_serial_read(args[0].into(), args[1]).await,
            // 创建任务
            SysCall::TaskCreate => self.sys_task_create(args[0].into(), args[1], args[2]).await,
            SysCall::TaskDestory => todo!(),
            // 退出任务
            SysCall::TaskExit => self.sys_task_exit(),
            // 获取当前任务 id
            SysCall::TaskSelf => Ok(self.get_task_id()),
            // 申请物理内存
            SysCall::PMAlloc => self.sys_pm_alloc(args[0], args[1], args[2]),
            // 映射内存
            SysCall::VMMap => self.sys_vm_map(args[0], args[1], args[2], args[3]),
            // 取消映射内存
            SysCall::VMUnmap => self.sys_vm_unmap(args[0], args[1]),
            SysCall::IrqListen => todo!(),
            SysCall::IrqUnlisten => todo!(),
            // 设置定时器，单位 ms
            SysCall::Time => self.sys_time(args[0]),
            // 获取当前系统时间
            SysCall::UPTime => self.sys_uptime(),
            SysCall::HinaVM => todo!(),
            // 关闭系统
            SysCall::Shutdown => self.sys_shutdown(),
            // 翻译虚拟地址
            SysCall::TransVAddr => self.sys_trans_paddr(args[0]),
        }
    }
}
