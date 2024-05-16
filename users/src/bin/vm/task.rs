use alloc::string::String;

/// Root Server 中的任务结构，主要进行任务的管理
/// 包含任务运行时的页表申请和缺页处理
pub struct Task {
    /// 任务 ID
    pub tid: usize,
    pub pager: usize,
    /// 任务名称
    pub name: String,
    /// 当前使用的最大的虚拟地址
    pub valloc_next: usize,
    /// 等待服务注册的注册名
    pub waiting_for: String,
    /// 是否监控任务完成情况
    pub watch_tasks: bool
}
