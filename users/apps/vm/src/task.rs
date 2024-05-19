use core::{arch::global_asm, cmp};

use alloc::{string::String, vec::Vec};
use spin::{Lazy, Mutex};
use syscall_consts::{Message, MessageContent, PageFaultReason};
use users::{
    align_down, align_up,
    syscall::{ipc_reply, sys_pm_alloc, sys_task_create, sys_vm_map, sys_vm_unmap, task_self},
    UserError, PAGE_SIZE,
};
use xmas_elf::{program::Type, ElfFile};

/// 引入 app 的 elf 文件
macro_rules! include_app {
    ($container:expr, $t:ident) => {
        $container.push((stringify!($t), {
            unsafe {
                core::slice::from_raw_parts(
                    concat_idents!(bin_, $t, _start) as *const u8,
                    concat_idents!(bin_, $t, _end) as usize
                        - concat_idents!(bin_, $t, _start) as usize,
                )
            }
        }))
    };
}

// 引入 elf 文件并且设置对齐
#[cfg(target_arch = "riscv64")]
global_asm!(
    r#"
        .p2align 12
        .global bin_shell_start
        .global bin_shell_end
        bin_shell_start:
        .incbin "target/riscv64gc-unknown-none-elf/release/shell"
        bin_shell_end:

        bin_pong_start:
        .incbin "target/riscv64gc-unknown-none-elf/release/pong"
        bin_pong_end:

        bin_blk_device_start:
        .incbin "target/riscv64gc-unknown-none-elf/release/blk_device"
        bin_blk_device_end:

        bin_ram_disk_start:
        .incbin "target/riscv64gc-unknown-none-elf/release/ram_disk"
        bin_ram_disk_end:

        bin_fs_start:
        .incbin "target/riscv64gc-unknown-none-elf/release/fs"
        bin_fs_end:
    "#,
);

#[cfg(target_arch = "aarch64")]
global_asm!(
    r#"
        .p2align 12
        .global bin_shell_start
        .global bin_shell_end
        bin_shell_start:
        .incbin "target/aarch64-unknown-none-softfloat/release/shell"
        bin_shell_end:

        bin_pong_start:
        .incbin "target/aarch64-unknown-none-softfloat/release/pong"
        bin_pong_end:

        bin_blk_device_start:
        .incbin "target/aarch64-unknown-none-softfloat/release/blk_device"
        bin_blk_device_end:

        bin_ram_disk_start:
        .incbin "target/aarch64-unknown-none-softfloat/release/ram_disk"
        bin_ram_disk_end:

        bin_fs_start:
        .incbin "target/aarch64-unknown-none-softfloat/release/fs"
        bin_fs_end:
    "#,
);

#[cfg(target_arch = "x86_64")]
global_asm!(
    r#"
        .p2align 12
        .global bin_shell_start
        .global bin_shell_end
        bin_shell_start:
        .incbin "target/x86_64-unknown-none/release/shell"
        bin_shell_end:

        bin_pong_start:
        .incbin "target/x86_64-unknown-none/release/pong"
        bin_pong_end:

        bin_blk_device_start:
        .incbin "target/x86_64-unknown-none/release/blk_device"
        bin_blk_device_end:

        bin_ram_disk_start:
        .incbin "target/x86_64-unknown-none/release/ram_disk"
        bin_ram_disk_end:

        bin_fs_start:
        .incbin "target/x86_64-unknown-none/release/fs"
        bin_fs_end:
    "#,
);

#[cfg(target_arch = "loongarch64")]
global_asm!(
    r#"
        .p2align 12
        .global bin_shell_start
        .global bin_shell_end
        bin_shell_start:
        .incbin "target/loongarch64-unknown-none/release/shell"
        bin_shell_end:

        bin_pong_start:
        .incbin "target/loongarch64-unknown-none/release/pong"
        bin_pong_end:

        bin_blk_device_start:
        .incbin "target/loongarch64-unknown-none/release/blk_device"
        bin_blk_device_end:

        bin_ram_disk_start:
        .incbin "target/loongarch64-unknown-none/release/ram_disk"
        bin_ram_disk_end:

        bin_fs_start:
        .incbin "target/loongarch64-unknown-none/release/fs"
        bin_fs_end:
    "#,
);

/// 临时页表，占位，为了方便处理
#[link_section = ".bss.page_data"]
static mut TMP_PAGE: [u8; PAGE_SIZE] = [0u8; PAGE_SIZE];

/// 获取临时页表的地址
#[inline]
pub fn tmp_page_addr() -> usize {
    unsafe { TMP_PAGE.as_mut_ptr() as usize }
}

/// 获取临时页表数组
#[inline]
pub fn tmp_page_buffer() -> &'static mut [u8] {
    unsafe { core::slice::from_raw_parts_mut(tmp_page_addr() as _, PAGE_SIZE) }
}

/// 引入所有 elf 文件的 bin 文件
static SERVERS_BIN: Lazy<Vec<(&str, &[u8])>> = Lazy::new(|| {
    let mut container = Vec::new();
    // extern_apps!(shell);

    extern "C" {
        fn bin_shell_start();
        fn bin_shell_end();
        fn bin_pong_start();
        fn bin_pong_end();
        fn bin_blk_device_start();
        fn bin_blk_device_end();
        fn bin_ram_disk_start();
        fn bin_ram_disk_end();
        fn bin_fs_start();
        fn bin_fs_end();
    }
    include_app!(container, shell);
    include_app!(container, pong);
    include_app!(container, blk_device);
    include_app!(container, ram_disk);
    include_app!(container, fs);
    container
});

/// Root Server 中的任务结构，主要进行任务的管理
/// 包含任务运行时的页表申请和缺页处理
pub struct Task {
    /// 任务 ID
    pub tid: usize,
    /// 页处理程序
    pub pager: usize,
    /// 当前 elf 文件
    pub file: &'static [u8],
    /// Elf 文件头
    pub elf_file: ElfFile<'static>,
    /// 任务名称
    pub name: String,
    /// 当前使用的最大的虚拟地址
    pub valloc_next: usize,
    /// 等待服务注册的注册名
    pub waiting_for: String,
    /// 是否监控任务完成情况
    pub watch_tasks: bool,
}

/// 微内核服务
#[derive(Debug, Clone)]
pub struct Service {
    /// 服务名称
    pub name: String,
    /// 服务对应的任务 ID
    pub task_id: usize,
}

impl Task {
    /// 处理页表错误
    pub fn handle_page_fault(
        &self,
        uaddr: usize,
        ip: usize,
        fault: PageFaultReason,
    ) -> Result<(), UserError> {
        // 第一个页通常不会使用，如果错误的位置为 0, 则用户态无法处理
        if uaddr < PAGE_SIZE {
            println!("[WARN] task {} access {:#x} @ {:#x}", self.tid, uaddr, ip);
            return Err(UserError::NotAllowed);
        }

        // FIXME: x86_64 will have present flags, need to fix
        // 如果页已经被映射了，那么可能是权限错误，当前程序无法处理
        // if fault.contains(PageFaultReason::PRESENT) {
        //     return Err(UserError::NotAllowed);
        // }

        let paddr = sys_pm_alloc(self.tid, PAGE_SIZE, 0) as usize;
        let vaddr = align_down(uaddr, PAGE_SIZE);

        self.elf_file.program_iter().for_each(|x| {
            // 只处理 LOAD 段信息
            if x.get_type().unwrap_or(Type::Null) != Type::Load {
                return;
            }

            // 获取段起始和结束信息
            let start = x.virtual_addr() as usize;
            let end = x.virtual_addr() as usize + x.file_size() as usize;

            // 如果不是这个段发生了问题，直接跳过
            if !(uaddr >= start && uaddr <= end) {
                return;
            }

            assert!(
                tmp_page_addr() % PAGE_SIZE == 0,
                "tmp_page not aligned by 4096"
            );

            // 获取段偏移信息和读取的大小
            let offset: usize = x.offset() as usize + vaddr - start;
            let rsize = cmp::min(end - vaddr, PAGE_SIZE);

            // 取消映射临时内存
            sys_vm_unmap(task_self(), tmp_page_addr());

            // TODO: use attrs to control privilege
            sys_vm_map(task_self(), tmp_page_addr(), paddr, 0);

            // 复制文件内容到 buffer 中
            tmp_page_buffer()[..rsize].copy_from_slice(&self.file[offset..offset + rsize]);
        });

        // TODO: use attrs to control privilege
        sys_vm_map(self.tid, uaddr, paddr, 0);

        Ok(())
    }

    /// 申请虚拟内存
    pub fn alloc_size(&mut self, size: usize) -> usize {
        // 确保申请的内存都是 4K 对齐的
        assert!(size % PAGE_SIZE == 0);
        // 移动 valloc_next 指针
        self.valloc_next += size;
        self.valloc_next - size
    }
}

/// 任务队列
pub static TASK_LIST: Mutex<Vec<Task>> = Mutex::new(Vec::new());
/// 服务列表
pub static SERVICE_LIST: Mutex<Vec<Service>> = Mutex::new(Vec::new());

/// 注册一个服务
pub fn register_service(tid: usize, name: String) {
    // 将服务加入到服务列表中
    SERVICE_LIST.lock().push(Service {
        name: name.clone(),
        task_id: tid,
    });
    // 如果有服务正在等待该服务，那么恢复其任务
    TASK_LIST.lock().iter_mut().for_each(|x| {
        // 如果有任务正在等待该服务，将其唤醒
        if x.waiting_for == name {
            x.waiting_for = String::new();
            let mut message = Message::blank();
            message.content = MessageContent::ServiceLookupReplyMsg(x.tid);
            ipc_reply(message.source, &mut message);
        }
    });
}

/// 启动 servers
pub fn spawn_servers() {
    SERVERS_BIN.iter().for_each(|&(name, server)| {
        // 读取 elf 文件
        let elf_file = xmas_elf::ElfFile::new(server).expect("can't find a valid elf file");
        let new_tid = sys_task_create(name, elf_file.header.pt2.entry_point() as _, task_self());
        // 如果 tid < 0，那么说明这个 task 没有启动起来
        if new_tid < 0 {
            println!(
                "task creation failed because: {:?}",
                UserError::try_from(new_tid)
            );
            return;
        }
        println!("spawn task {} id {}", name, new_tid);
        // 迭代段信息，找到利用的最大的虚拟地址
        let mut valloc_next = 0;
        elf_file.program_iter().for_each(|x| {
            // 只处理 LOAD 段信息
            if x.get_type().unwrap_or(Type::Null) != Type::Load {
                return;
            }
            let end = align_up((x.virtual_addr() + x.mem_size()) as usize, PAGE_SIZE);

            // 设置最大的虚拟内存地址
            valloc_next = cmp::max(end, valloc_next);
        });
        // 将新任务添加到队列中
        TASK_LIST.lock().push(Task {
            tid: new_tid as usize,
            pager: task_self(),
            file: &server,
            elf_file,
            name: String::from(name),
            valloc_next,
            waiting_for: String::new(),
            watch_tasks: false,
        });
    });
}
