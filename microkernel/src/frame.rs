use alloc::vec::Vec;
use buddy_system_allocator::LockedFrameAllocator;
use log::info;
use polyhal::{addr::PhysPage, PAGE_SIZE, VIRT_ADDR_START};
use spin::Lazy;

use crate::utils::align_up;

static LOCK_FRAME_ALLOCATOR: Lazy<LockedFrameAllocator<32>> =
    Lazy::new(|| LockedFrameAllocator::new());

pub fn add_frame_range(mm_start: usize, mm_end: usize) {
    extern "C" {
        fn end();
    }
    let mut frame_start = mm_start / PAGE_SIZE;
    let frame_end = mm_end / PAGE_SIZE;
    let kernel_end = align_up(end as usize & (!VIRT_ADDR_START), PAGE_SIZE) / PAGE_SIZE;
    if frame_start <= kernel_end && kernel_end <= frame_end {
        frame_start = kernel_end;
    }
    info!("add memory range: {:#x} - {:#x}", frame_start, frame_end);
    LOCK_FRAME_ALLOCATOR
        .lock()
        .add_frame(frame_start, frame_end);
}

pub fn frame_alloc_persist() -> PhysPage {
    LOCK_FRAME_ALLOCATOR
        .lock()
        .alloc(1)
        .map(PhysPage::new)
        .inspect(|x| x.drop_clear())
        .expect("can't find memory page")
}

/// 申请页表
pub fn frame_alloc(pages: usize) -> Vec<FrameTracker> {
    let mut ret = Vec::new();
    LOCK_FRAME_ALLOCATOR.lock().alloc(pages).inspect(|x| {
        for i in 0..pages {
            ret.push(FrameTracker(PhysPage::new(*x + i)))
        }
    });
    ret
}

pub fn frame_dealloc(ppn: PhysPage) {
    LOCK_FRAME_ALLOCATOR.lock().dealloc(ppn.as_num(), 1);
}

pub struct FrameTracker(pub PhysPage);

impl Drop for FrameTracker {
    fn drop(&mut self) {
        frame_dealloc(self.0)
    }
}
