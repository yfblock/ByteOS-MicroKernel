#![no_std]
#![no_main]
#![feature(panic_info_message)]

extern crate alloc;
extern crate allocator;
use core::sync::atomic::{AtomicBool, Ordering};

use executor::{DEFAULT_EXECUTOR, TASK_MAP};
use log::info;
use polyhal::{get_cpu_num, get_mem_areas, TrapFrame, TrapType, VIRT_ADDR_START};
use spin::Mutex;
use syscall_consts::PageFaultReason;
use task::{current_microkernel_task, MicroKernelTask};

pub mod async_ops;
pub mod consts;
mod frame;
#[macro_use]
mod lang_items;
mod syscall;
mod task;
mod utils;

#[polyhal::arch_interrupt]
fn interrupt_handler(tf: TrapFrame, trap_type: TrapType) {
    match trap_type {
        // UserEnvCall 不会在这里处理，会在 Async function 内部处理。
        TrapType::UserEnvCall => {}
        TrapType::Time => {
            // 检查所有的 TimeOut，如果时间到达，那么创建 TIMER Notification 并清空 TimeOut
            TASK_MAP
                .lock()
                .values()
                .filter_map(|x| x.upgrade())
                .filter_map(|x| x.downcast_arc::<MicroKernelTask>().ok())
                .for_each(|x| x.check_timeout());
        }
        TrapType::InstructionPageFault(vaddr) => {
            current_microkernel_task().inspect(|x| x.set_fault(vaddr, PageFaultReason::EXEC));
        }
        TrapType::StorePageFault(vaddr) => {
            current_microkernel_task().inspect(|x| x.set_fault(vaddr, PageFaultReason::WRITE));
        }
        TrapType::LoadPageFault(vaddr) => {
            current_microkernel_task().inspect(|x| x.set_fault(vaddr, PageFaultReason::READ));
        }
        TrapType::IllegalInstruction(vaddr) => {
            panic!("illegal instruction @ {:#x} {:#x?}", vaddr, tf);
        }
        _ => {
            log::debug!("trap {:#x?}", trap_type);
        }
    }
}

/// 程序入口
#[polyhal::arch_entry]
fn main(hart_id: usize) {
    // 判断是否为 BOOT 核心，原子变量
    static BOOT_CORE: AtomicBool = AtomicBool::new(true);
    // 采用原子操作进行 compare and excahnge，所以只有一个核心是主核心
    if BOOT_CORE
        .compare_exchange(true, false, Ordering::Acquire, Ordering::Relaxed)
        .is_ok()
    {
        // 设置 log 级别
        lang_items::init(Some(option_env!("LOG").unwrap_or("debug")));
        println!("Boot @ {}", hart_id);
        // 初始化堆分配器
        allocator::init();
        // 初始化 polyhal
        polyhal::init(&lang_items::PageAlloImpl);

        // 遍历所有物理内存区域，并添加到内存分配器中
        get_mem_areas().into_iter().for_each(|(start, size)| {
            let start = start & (!VIRT_ADDR_START);
            println!("Detected Memory {:#x} - {:#x}", start, start + size);
            println!("VIRT_ADDR:{:#x}", VIRT_ADDR_START);
            frame::add_frame_range(start, start + size);
        });

        // Initialize the default async executor
        DEFAULT_EXECUTOR.init(get_cpu_num());

        // Add the root server to the async executor
        info!("Add root server to the async executor");
        task::add_root_server();

        // Boot all cores
        polyhal::multicore::MultiCore::boot_all();
        info!("boot all cores finished");

        // Run tasks
        DEFAULT_EXECUTOR.run();
    } else {
        println!("Hart {} is not boot core", hart_id);
    }

    // 下面的代码理论上在所有和核心都参与调度的时候执行不到，仅供调试使用
    // 完成的核心数
    static FINISHED_CORES: Mutex<usize> = Mutex::new(0);
    // 如果当前核心完成该任务则 +1
    *FINISHED_CORES.lock() += 1;
    // 如果所有核心都完成了，则退出
    loop {
        if *FINISHED_CORES.lock() == get_cpu_num() {
            break;
        }
        polyhal::wfi();
    }
}
