use core::arch::global_asm;

use alloc::{collections::VecDeque, string::String, sync::Arc, vec::Vec};
use executor::{task::TaskType, task_id_alloc, thread::spawn, AsyncTask, TaskId};
use log::info;
use polyhal::{
    addr::{PhysPage, VirtPage},
    pagetable::{MappingFlags, MappingSize, PageTable, PageTableWrapper},
    run_user_task, TrapFrame, TrapFrameArgs, PAGE_SIZE, VIRT_ADDR_START,
};
use spin::mutex::Mutex;
use xmas_elf::program::Type;

use crate::{
    consts::{USER_STACK_PAGES, USER_STACK_TOP_ADDR},
    frame::{frame_alloc, FrameTracker},
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

pub enum TaskState {
    UnUsed,
    Runable,
    Blocked,
}

pub struct MicroKernelTask {
    /// 任务中断上下文
    trap_frame: TrapFrame,
    /// 任务页表
    page_table: PageTableWrapper,
    /// 页表代理任务
    pager: Option<Arc<MicroKernelTask>>,
    /// 任务 ID
    tid: TaskId,
    /// 任务名称
    name: String,
    /// 任务状态
    state: TaskState,
    /// 任务是否被删除
    pub destoryed: Mutex<bool>,
    /// 剩余超时时间
    timeout: usize,
    /// 等待向此 `TASK` 发送消息的任务 ID 队列
    senders: VecDeque<TaskId>,
    /// 可以向此 `TASK` 发送消息的任务 ID
    wait_for: TaskId,
    pages: Vec<FrameTracker>,
    /// 消息暂存区，因为同时只有一个任务可以向此任务发送消息
    /// 所以可以只需要一个 message 即可，而不需要一个队列
    message: Option<Message>,
}

impl AsyncTask for MicroKernelTask {
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

pub fn add_root_server() {
    // 创建 ROOT_SERVER 任务
    let mut root_server = MicroKernelTask {
        trap_frame: TrapFrame::new(),
        page_table: PageTableWrapper::alloc(),
        pager: None,
        tid: task_id_alloc(),
        name: String::from("VM"),
        state: TaskState::Runable,
        destoryed: Mutex::new(false),
        timeout: 0,
        senders: VecDeque::new(),
        wait_for: 0,
        pages: Vec::new(),
        message: None,
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
    let root_server_elf = unsafe { core::slice::from_raw_parts(start as *const u8, end - start) };
    let elf_header = xmas_elf::ElfFile::new(root_server_elf)
        .expect("can't get a correct elf file as root server");
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

        // hexdump(unsafe {
        //     core::slice::from_raw_parts((ppn.to_addr() + VIRT_ADDR_START) as _ , x.mem_size() as usize)
        // }, x.virtual_addr() as usize);

        // 映射当前内存
        for i in 0..pages {
            info!("map {:?} -> {:?}", vpn + i, ppn + i);
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
        let page = frame_alloc().expect("can't allocate page for root server at boot stage.");
        let stack_top = VirtPage::from_addr(USER_STACK_TOP_ADDR - i * PAGE_SIZE);
        root_server.page_table.map_page(
            stack_top,
            page.0,
            MappingFlags::URWX,
            MappingSize::Page4KB,
        );
        root_server.pages.push(page);
    }
    info!(
        "Root server entry point: {:#x}",
        elf_header.header.pt2.entry_point()
    );
    root_server.trap_frame[TrapFrameArgs::SEPC] = elf_header.header.pt2.entry_point() as _;
    root_server.trap_frame[TrapFrameArgs::SP] = USER_STACK_TOP_ADDR;

    let root_server = Arc::new(root_server);
    spawn(
        root_server.clone(),
        MicroKernelTask::run(root_server.clone()),
    )
}

impl MicroKernelTask {
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

    pub async fn run(task: Arc<MicroKernelTask>) {
        let tf = task.get_trap_frame();
        loop {
            // 如果任务已经退出了那么，需要退出循环，函数结束后，会由 Rust 回收内存
            if *task.destoryed.lock() == true {
                break;
            }
            // 如果运行的结果为 Some(()), 那么此次是被 syscall 打断的, 否则是其他原因
            if let Some(_) = run_user_task(tf) {
                let res = task.syscall(tf[TrapFrameArgs::SYSCALL], tf.args()).await;
                tf.syscall_ok();
                match res {
                    Ok(res) => tf[TrapFrameArgs::RET] = res,
                    Err(err) => tf[TrapFrameArgs::RET] = err as usize,
                }
            }
        }
        info!("task {} exited successfully", task.get_task_id());
    }
}
