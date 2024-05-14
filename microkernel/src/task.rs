use alloc::{collections::VecDeque, string::String, vec::Vec};
use executor::{task::TaskType, AsyncTask, TaskId};
use polyhal::{pagetable::PageTableWrapper, TrapFrame};

use crate::frame::FrameTracker;

pub enum TaskState {}

pub struct Task {
    /// 任务中断上下文
    trap_frame: TrapFrame,
    /// 任务页表
    page_table: PageTableWrapper,
    /// 任务 ID
    tid: TaskId,
    /// 任务名称
    name: String,
    /// 任务状态
    state: TaskState,
    /// 任务是否被删除
    destoryed: bool,
    /// 剩余超时时间
    timeout: usize,

    /// 等待向此 `TASK` 发送消息的任务 ID 队列
    senders: VecDeque<TaskId>,
    /// 可以向此 `TASK` 发送消息的任务 ID
    wait_for: TaskId,
    pages: Vec<FrameTracker>,
    /// 消息暂存区，因为同时只有一个任务可以向此任务发送消息
    /// 所以可以只需要一个 message 即可，而不需要一个队列
    message: Message,
}

impl AsyncTask for Task {
    fn get_task_id(&self) -> TaskId {
        self.tid
    }

    fn before_run(&self) {
        self.page_table.0.change();
    }

    fn get_task_type(&self) -> TaskType {
        TaskType::MicroTask
    }

    fn exit(&self, _exit_code: usize) {
        todo!("Exit MicroKernel Task")
    }

    fn exit_code(&self) -> Option<usize> {
        todo!("Get Microkernel Task Exit Code")
    }
}

pub struct Message {}
