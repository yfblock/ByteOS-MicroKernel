use core::arch::global_asm;

use alloc::{string::String, sync::Arc, vec::Vec};
use executor::{
    current_task, task::TaskType, task_id_alloc, thread::spawn, yield_now, AsyncTask, TaskId,
};
use log::info;
use polyhal::{
    addr::{PhysPage, VirtAddr, VirtPage},
    pagetable::{MappingFlags, MappingSize, PageTable, PageTableWrapper},
    run_user_task,
    time::Time,
    TrapFrame, TrapFrameArgs, PAGE_SIZE, VIRT_ADDR_START,
};
use spin::mutex::Mutex;
use syscall_consts::{
    ExceptionType, IPCFlags, Message, MessageContent, Notify, NotifyEnum, PMAllocFlags,
    PageFaultReason, IPC_ANY,
};
use xmas_elf::program::Type;

use crate::{
    consts::{USER_STACK_PAGES, USER_STACK_TOP_ADDR},
    frame::{frame_alloc, FrameTracker},
    utils::align_up,
};

// 包含 vm elf 文件，vm server 将作为 root server 运行。
// static ROOT_SERVER_BIN: &'static [u8] = include_bytes!("../../users/target/riscv64gc-unknown-none-elf/release/vm");
global_asm!(
    r#"
    .p2align 12
    ROOT_SERVER_BIN:
    .incbin "users/target/riscv64gc-unknown-none-elf/release/vm"
    ROOT_SERVER_END:
"#
);

/// 任务状态
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TaskState {
    /// 任务没有被使用
    UnUsed,
    /// 任务可以运行
    Runable,
    /// 任务被阻塞
    Blocked,
}

pub struct MicroKernelTask {
    /// 任务中断上下文
    pub trap_frame: TrapFrame,
    /// 任务页表
    pub page_table: PageTableWrapper,
    /// 页表代理任务
    pub pager: Option<Arc<MicroKernelTask>>,
    /// 任务 ID
    pub tid: TaskId,
    /// 任务名称
    pub name: String,
    /// 任务状态
    pub state: Mutex<TaskState>,
    /// 任务是否被删除
    pub destoryed: Mutex<bool>,
    /// 剩余超时时间
    pub timeout: Mutex<usize>,
    /// 当前等待处理的通知
    pub notifications: Mutex<Notify>,
    /// 等待向此 `TASK` 发送消息的任务 ID 队列
    pub senders: Mutex<Vec<TaskId>>,
    /// 可以向此 `TASK` 发送消息的任务 ID
    pub wait_for: Mutex<Option<TaskId>>,
    /// 当前任务拥有的 pages
    pub pages: Mutex<Vec<FrameTracker>>,
    /// 消息暂存区，因为同时只有一个任务可以向此任务发送消息
    /// 所以可以只需要一个 message 即可，而不需要一个队列
    pub message: Mutex<Option<Message>>,
    /// 页表错误，由于采用异步形式，但是发生错误的时候需要发送并等待 IPC
    /// 所以发生错误的时候可以存在在这个结构中，进入 async 函数后处理
    pub fault: Mutex<Option<(usize, usize, PageFaultReason)>>,
}

impl AsyncTask for MicroKernelTask {
    /// 获取当前任务 ID
    fn get_task_id(&self) -> TaskId {
        self.tid
    }

    /// 会在任务执行前被调用
    /// 这里会执行切换页表的操作
    fn before_run(&self) {
        self.page_table.0.change();
    }

    /// 获取任务类型，这里恒返回 [TaskType::MicroTask]
    /// 这个结构是由于使用了 `ByteOS` 中 [executor] 的 [AsyncTask]
    /// 当然采用这种设计可能是可以支持混合内核，
    /// 让 `Unikernel`, `MicroKernel`, `MonotonicKernel` 运行在同一块 CPU 上
    /// 也许可以期待一下
    fn get_task_type(&self) -> TaskType {
        TaskType::MicroTask
    }

    /// 退出当前任务
    fn exit(&self, _exit_code: usize) {
        todo!("Exit MicroKernel Task")
    }

    /// 获取当前任务的退出码
    fn exit_code(&self) -> Option<usize> {
        todo!("Get Microkernel Task Exit Code")
    }
}

/// 将 ROOT_SERVER 任务添加到调度器中
pub fn add_root_server() {
    // 创建 ROOT_SERVER 任务
    let mut root_server = MicroKernelTask {
        trap_frame: TrapFrame::new(),
        page_table: PageTableWrapper::alloc(),
        pager: None,
        tid: task_id_alloc(),
        name: String::from("VM"),
        state: Mutex::new(TaskState::Runable),
        destoryed: Mutex::new(false),
        timeout: Mutex::new(0),
        notifications: Mutex::new(Notify::new()),
        senders: Mutex::new(Vec::new()),
        wait_for: Mutex::new(None),
        pages: Mutex::new(Vec::new()),
        message: Mutex::new(None),
        fault: Mutex::new(None),
    };
    // 切换到 ROOT_SERVER 的页表，方便进行内存复制和切换，以及映射新的内存
    root_server.page_table.change();

    extern "C" {
        fn ROOT_SERVER_BIN();
        fn ROOT_SERVER_END();
    }
    let start = ROOT_SERVER_BIN as usize;
    let end = ROOT_SERVER_END as usize;
    info!("root server memory area: {:#x} - {:#x}", start, end);
    // 将 ROOT_SERVER 的地址转换为数组
    let root_server_elf = unsafe { core::slice::from_raw_parts(start as *const u8, end - start) };
    // 获取 ROOT_SERVER 头信息
    let elf_header = xmas_elf::ElfFile::new(root_server_elf)
        .expect("can't get a correct elf file as root server");
    // 处理 ROOT_SERVER 的段信息
    elf_header.program_iter().for_each(|x| {
        // 如果不是 LOAD 段，直接跳过
        if x.get_type().unwrap_or(Type::Null) != Type::Load {
            return;
        }
        log::debug!(
            "program_header: {:?} addr: {:#x}, size: {:#x} offset: {:#x}",
            x.get_type().unwrap(),
            x.virtual_addr(),
            x.mem_size(),
            x.offset()
        );
        // 当前段的虚拟页表号
        let vpn = VirtPage::from_addr(x.virtual_addr() as usize);
        // 当前段的物理页表号
        let ppn = PhysPage::from_addr(start - VIRT_ADDR_START + x.offset() as usize);
        // 当前段需要的页表数量
        let pages = (x.mem_size() as usize + PAGE_SIZE - 1) / PAGE_SIZE;

        // 映射当前内存
        for i in 0..pages {
            root_server.page_table.map_page(
                vpn + i,
                ppn + i,
                MappingFlags::URWX,
                MappingSize::Page4KB,
            );
        }
    });

    // 申请新的栈页表, 4KB * 20 = 800KB
    for i in 0..USER_STACK_PAGES {
        let page = frame_alloc(1);
        assert!(
            page.len() > 0,
            "can't allocate page for root server at boot stage."
        );

        // 获取栈地址
        let stack_addr = VirtPage::from_addr(USER_STACK_TOP_ADDR - i * PAGE_SIZE);

        // 映射栈内存
        root_server.page_table.map_page(
            stack_addr,
            page[0].0,
            MappingFlags::URWX,
            MappingSize::Page4KB,
        );
        root_server.pages.lock().extend(page);
    }
    info!(
        "Root server entry point: {:#x}",
        elf_header.header.pt2.entry_point()
    );

    // 设置 ROOT_SERVER 的中断上下文，包括入口和栈
    root_server.trap_frame[TrapFrameArgs::SEPC] = elf_header.header.pt2.entry_point() as _;
    root_server.trap_frame[TrapFrameArgs::SP] = USER_STACK_TOP_ADDR;

    // 将 ROOT_SERVER 加入到调度器中
    let root_server = Arc::new(root_server);
    spawn(root_server.clone(), root_server.run())
}

impl MicroKernelTask {
    /// 创建新的任务
    pub fn new(name: &str, entry_point: usize, pager: Option<Arc<MicroKernelTask>>) -> TaskId {
        // 申请新的任务 ID
        let new_tid = task_id_alloc();

        // 创建新的任务
        let mut new_task = MicroKernelTask {
            trap_frame: TrapFrame::new(),
            page_table: PageTableWrapper::alloc(),
            pager,
            tid: new_tid,
            name: String::from(name),
            state: Mutex::new(TaskState::UnUsed),
            destoryed: Mutex::new(false),
            timeout: Mutex::new(0),
            notifications: Mutex::new(Notify::new()),
            senders: Mutex::new(Vec::new()),
            wait_for: Mutex::new(None),
            pages: Mutex::new(Vec::new()),
            message: Mutex::new(None),
            fault: Mutex::new(None),
        };

        // 设置任务上下文
        new_task.trap_frame[TrapFrameArgs::SEPC] = entry_point;
        new_task.trap_frame[TrapFrameArgs::SP] = USER_STACK_TOP_ADDR;

        // 申请新的栈页表, 4KB * 20 = 800KB
        for i in 0..USER_STACK_PAGES {
            let page = frame_alloc(1);
            assert!(
                page.len() > 0,
                "can't allocate page for root server at boot stage."
            );
            // 获取栈地址
            let stack_addr = VirtPage::from_addr(USER_STACK_TOP_ADDR - i * PAGE_SIZE);
            // 映射栈内存
            new_task.page_table.map_page(
                stack_addr,
                page[0].0,
                MappingFlags::URWX,
                MappingSize::Page4KB,
            );
            new_task.pages.lock().extend(page);
        }

        // 将新的任务加入到调度器中
        let new_task = Arc::new(new_task);
        // 恢复当前任务的运行状态
        new_task.resume();
        // 将任务加入到任务队列中
        spawn(new_task.clone(), new_task.run());
        new_tid
    }

    /// 设置 fault field 以便后面处理
    pub fn set_fault(&self, vaddr: usize, mut reason: PageFaultReason) {
        let sepc = self.trap_frame[TrapFrameArgs::SEPC];
        if let Some(translated) = PageTable::current().translate(VirtAddr::new(vaddr)) {
            if !translated.1.is_empty() {
                reason |= PageFaultReason::PRESENT;
            }
        }
        if sepc <= VIRT_ADDR_START {
            reason |= PageFaultReason::USER;
        }

        *self.fault.lock() = Some((vaddr, sepc, reason));
    }

    /// 页表错误处理程序
    pub async fn handle_page_fault(&self) {
        let mut fault = self.fault.lock();
        if let Some((vaddr, sepc, reason)) = *fault {
            // 内核不会发生 page fault, 如果发生了直接 panic，人工处理
            if sepc >= VIRT_ADDR_START {
                panic!("can't trigger page fault in kernel {vaddr} @ {sepc}");
            }
            // 获取当前 pager
            if self.pager.is_none() {
                panic!("unexpected page fault in user task {}, it don't have a pager {vaddr:#x} @ {sepc:#x}", self.tid);
            }
            let pager = self.pager.clone().unwrap();
            // 设置 message
            let mut message = Message::blank();
            message.content = MessageContent::PageFault {
                tid: self.tid,
                uaddr: vaddr,
                ip: sepc,
                fault: reason,
            };
            // 发送 IPC
            let ret = self
                .ipc(
                    pager.tid,
                    pager.tid,
                    &mut message,
                    IPCFlags::CALL | IPCFlags::KERNEL,
                )
                .await;
            if ret.is_err() || message.content != MessageContent::PageFaultReply {
                // *self.exit(exit_code)
                panic!(
                    "task exit with exception: {:?}",
                    ExceptionType::InvalidPagerReply
                );
            }
            // Page Fault 处理完毕
            *fault = None;
        }
    }

    /// 获取 PageTable
    pub fn page_table(&self) -> PageTable {
        self.page_table.0
    }

    /// 获取 TrapFrame mutable reference
    pub fn get_trap_frame(&self) -> &mut TrapFrame {
        unsafe {
            (&self.trap_frame as *const _ as *mut TrapFrame)
                .as_mut()
                .unwrap()
        }
    }

    /// 给当前任务发送 Notification.
    pub fn notify(&self, notification: Notify) {
        // 如果当前任务正在等待 IPC, 那么直接通知给任务
        // 反之则将 通知放进通知队列中等待。
        if *self.state.lock() == TaskState::Blocked && *self.wait_for.lock() == Some(IPC_ANY) {
            *self.message.lock() = Some(Message {
                source: IPC_ANY,
                content: MessageContent::NotifyField {
                    notications: self.notifications.lock().pop_all() | notification,
                },
            });
            self.resume();
        } else {
            *self.notifications.lock() |= notification;
        }
    }

    /// 运行当前任务
    pub async fn run(self: Arc<MicroKernelTask>) {
        let tf = self.get_trap_frame();
        loop {
            // 如果任务已经退出了那么，需要退出循环，函数结束后，会由 Rust 回收内存
            if *self.destoryed.lock() == true {
                break;
            }
            // 如果任务被阻塞了，那么等待调度
            if *self.state.lock() != TaskState::Runable {
                yield_now().await;
                continue;
            }
            // 如果运行的结果为 Some(()), 那么此次是被 syscall 打断的, 否则是其他原因
            if let Some(_) = run_user_task(tf) {
                let res = self.syscall(tf[TrapFrameArgs::SYSCALL], tf.args()).await;
                tf.syscall_ok();
                match res {
                    Ok(res) => tf[TrapFrameArgs::RET] = res,
                    Err(err) => tf[TrapFrameArgs::RET] = err as usize,
                }
            }
            self.handle_page_fault().await;
        }
        log::trace!("task {} exited successfully", self.get_task_id());
    }

    /// 检查当前任务的 timeout, 一般会在 Block 状态下做
    pub fn check_timeout(&self) {
        let mut timeout = self.timeout.lock();
        if *timeout != 0 && Time::now().to_nsec() >= *timeout {
            *timeout = 0;
            drop(timeout);
            self.notify(NotifyEnum::TIMER.into());
        }
    }

    /// 阻塞当前任务
    pub fn block(&self) {
        *self.state.lock() = TaskState::Blocked;
    }

    /// 恢复程序的运行状态
    pub fn resume(&self) {
        *self.state.lock() = TaskState::Runable;
    }

    /// 申请物理内存
    pub fn alloc_memory(&self, size: usize, flags: PMAllocFlags) -> usize {
        let pages = frame_alloc(align_up(size, PAGE_SIZE) / PAGE_SIZE);
        let start = pages[0].0;
        // 清空所有页表
        if flags.contains(PMAllocFlags::ZEROD) {
            pages.iter().for_each(|x| x.0.drop_clear());
        }
        self.pages.lock().extend(pages);
        start.to_addr()
    }

    /// 映射内存
    pub fn map_page(&self, vpn: VirtPage, ppn: PhysPage) {
        log::debug!("map {:?} -> {:?}", vpn, ppn);
        self.page_table()
            .map_page(vpn, ppn, MappingFlags::URWX, MappingSize::Page4KB);
    }
}

/// 获取当前正在运行的 MicroKernel Task
pub fn current_microkernel_task() -> Option<Arc<MicroKernelTask>> {
    current_task().downcast_arc::<MicroKernelTask>().ok()
}
