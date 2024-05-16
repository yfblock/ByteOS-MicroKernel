use alloc::string::String;

/// 读取 target_dir，使用 macro_rule 可以保证能够在编译时访问
macro_rules! target_dir {() => (
    r"../../../target/riscv64gc-unknown-none-elf/release/"
)}

/// 引入 app 的 elf 文件
macro_rules! include_app {($t: expr) => (
    include_bytes!(concat!(target_dir!(), $t))
)}

/// 引入所有 elf 文件的 bin 文件
static SERVERS_BIN: &[&[u8]] = &[
    include_app!("/shell")
];

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

/// 启动 servers 
pub fn spawn_servers() {
    SERVERS_BIN.into_iter().for_each(|&server| {
        println!("server 1: {:#x} - {:#x}", server.as_ptr() as usize, server.as_ptr() as usize + server.len());
    });
}
