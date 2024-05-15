#![no_std]
#![no_main]
#![feature(panic_info_message)]

extern crate alloc;
extern crate allocator;
use core::sync::atomic::{AtomicBool, Ordering};

use executor::DEFAULT_EXECUTOR;
use log::info;
use polyhal::{get_cpu_num, get_mem_areas, TrapFrame, TrapType, VIRT_ADDR_START};
use spin::Mutex;

mod frame;
#[macro_use]
mod lang_items;
mod task;

#[polyhal::arch_interrupt]
fn interrupt_handler(_tf: TrapFrame, trap_type: TrapType) {
    log::trace!("trap {:#x?}", trap_type);
}

#[polyhal::arch_entry]
fn main(hart_id: usize) {
    static BOOT_CORE: AtomicBool = AtomicBool::new(true);
    if BOOT_CORE
        .compare_exchange(true, false, Ordering::Acquire, Ordering::Relaxed)
        .is_ok()
    {
        lang_items::init(Some("debug"));
        println!("Boot @ {}", hart_id);
        allocator::init();
        polyhal::init(&lang_items::PageAlloImpl);

        get_mem_areas().into_iter().for_each(|(start, size)| {
            let start = start & (!VIRT_ADDR_START);
            println!("Detected Memory {:#x} - {:#x}", start, start + size);
            println!("VIRT_ADDR:{:#x}", VIRT_ADDR_START);
            frame::add_frame_range(start, start + size);
        });

        // Initialize the default async executor
        DEFAULT_EXECUTOR.init(get_cpu_num());

        // Add the root service to the async executor
        info!("Add root service to the async executor");
        task::add_root_service();

        // Boot all cores
        polyhal::multicore::MultiCore::boot_all();
        info!("boot all cores finished");

        // Run tasks
        DEFAULT_EXECUTOR.run();
    } else {
        println!("Hart {} is not boot core", hart_id);
    }

    static FINISHED_CORES: Mutex<usize> = Mutex::new(0);
    *FINISHED_CORES.lock() += 1;
    loop {
        if *FINISHED_CORES.lock() == get_cpu_num() {
            break;
        }
        polyhal::wfi();
    }
}
