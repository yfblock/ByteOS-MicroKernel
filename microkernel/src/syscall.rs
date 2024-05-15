use polyhal::{shutdown, time::Time};
use syscall_consts::{SysCall, SysCallError};

use crate::{async_ops::sleep, lang_items::puts, task::MicroKernelTask, utils::UserBuffer};

type SysResult = Result<usize, SysCallError>;

impl MicroKernelTask {
    /// 串口输出
    pub async fn sys_serial_write(
        &self,
        buf: UserBuffer<u8>,
        buf_len: usize,
    ) -> Result<usize, SysCallError> {
        let bytes = buf.slice_mut_with_len(buf_len);
        puts(bytes);
        Ok(bytes.len())
    }

    /// 退出当前任务
    pub async fn sys_task_exit(&self) -> SysResult {
        *self.destoryed.lock() = true;
        Ok(0)
    }

    /// 休眠 ms
    pub async fn sys_time(&self, ms: usize) -> SysResult {
        // sleep(ms).await;
        *self.timeout.lock() += Time::now().to_nsec() + ms * 0x1000_000;
        Ok(0)
    }

    /// 关闭计算机
    pub async fn sys_shutdown(&self) -> ! {
        shutdown();
    }

    /// 处理系统调用
    pub async fn syscall(&self, id: usize, args: [usize; 6]) -> Result<usize, SysCallError> {
        match SysCall::from(id) {
            SysCall::IPC => todo!(),
            SysCall::Notify => todo!(),
            SysCall::SerialWrite => self.sys_serial_write(args[0].into(), args[1]).await,
            SysCall::SerialRead => todo!(),
            SysCall::TaskCreate => todo!(),
            SysCall::TaskDestory => todo!(),
            SysCall::TaskExit => self.sys_task_exit().await,
            SysCall::TaskSelf => todo!(),
            SysCall::PMAlloc => todo!(),
            SysCall::VMMap => todo!(),
            SysCall::VNUnmap => todo!(),
            SysCall::IrqListen => todo!(),
            SysCall::IrqUnlisten => todo!(),
            SysCall::Time => self.sys_time(args[0]).await,
            SysCall::UPTime => todo!(),
            SysCall::HinaVM => todo!(),
            SysCall::Shutdown => self.sys_shutdown().await,
            SysCall::Unknown => todo!(),
        }
    }
}
