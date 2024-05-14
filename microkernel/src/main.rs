#![no_std]
#![no_main]

extern crate alloc;
extern crate allocator;
use polyhal::{get_mem_areas, TrapFrame, TrapType, VIRT_ADDR_START};

mod frame;
#[macro_use]
mod lang_items;
mod task;

#[polyhal::arch_interrupt]
fn interrupt_handler(_tf: TrapFrame, trap_type: TrapType) {
    log::debug!("trap {:#x?}", trap_type);
}

#[polyhal::arch_entry]
fn main(hart_id: usize) {
    allocator::init();
    lang_items::init(Some("debug"));
    println!("Boot @ {}", hart_id);
    polyhal::init(&lang_items::PageAlloImpl);

    get_mem_areas().into_iter().for_each(|(start, size)| {
        let start = start & (!VIRT_ADDR_START);
        println!("Detected Memory {:#x} - {:#x}", start, start + size);
        println!("VIRT_ADDR:{:#x}", VIRT_ADDR_START);
        frame::add_frame_range(start, start + size);
    });
}
