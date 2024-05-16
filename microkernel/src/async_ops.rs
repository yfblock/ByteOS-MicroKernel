use core::{
    future::Future,
    pin::Pin,
    task::{Context, Poll},
};

use polyhal::time::Time;

use crate::task::{MicroKernelTask, TaskState};

/// 等待特定的 time, 单位 ms
pub struct NextTime(pub usize);

/// 为 [NextTime] 实现 Trait
impl Future for NextTime {
    type Output = ();

    fn poll(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Self::Output> {
        if Time::now().to_msec() >= self.0 {
            Poll::Ready(())
        } else {
            Poll::Pending
        }
    }
}

/// 内核 sleeping
#[inline]
pub async fn sleep(ms: usize) {
    NextTime(Time::now().to_msec() + ms).await;
}

/// 等待系统 IPC 消息
pub struct WaitIPC<'a>(pub &'a MicroKernelTask);

/// 为 [WaitIPC] 实现 Future
impl<'a> Future for WaitIPC<'a> {
    type Output = ();

    fn poll(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Self::Output> {
        match self.0.wait_for.lock().is_some() {
            // 仍在 Wait for
            true => Poll::Pending,
            false => Poll::Ready(()),
        }
    }
}

/// 等待系统恢复为 [TaskState::Runable] 状态
pub struct WaitResume<'a>(pub &'a MicroKernelTask);

/// 为 [WaitResume] 实现 Future
impl<'a> Future for WaitResume<'a> {
    type Output = ();

    fn poll(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Self::Output> {
        self.0.check_timeout();
        match *self.0.state.lock() == TaskState::Runable {
            // 任务可以运行了
            true => Poll::Ready(()),
            false => Poll::Pending,
        }
    }
}
