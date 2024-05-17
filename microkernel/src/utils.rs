use core::{marker::PhantomData, str::Utf8Error};

use alloc::string::{String, ToString};
use polyhal::{
    addr::VirtAddr,
    pagetable::{MappingFlags, PageTable},
};
use syscall_consts::PageFaultReason;

use crate::task::MicroKernelTask;

/// 将 `value` 根据 `align` 上对齐
#[allow(dead_code)]
pub fn align_up(value: usize, align: usize) -> usize {
    (value + align - 1) / align * align
}

#[derive(Debug, Clone, Copy)]
pub struct UserBuffer<T> {
    addr: VirtAddr,
    r#type: PhantomData<T>,
}

impl<T> From<usize> for UserBuffer<T> {
    fn from(value: usize) -> Self {
        UserBuffer {
            addr: VirtAddr::new(value),
            r#type: PhantomData,
        }
    }
}

/// 判断虚拟地址在当前页表中是否被映射
#[inline]
pub fn is_mapped(vaddr: VirtAddr) -> bool {
    PageTable::current()
        .translate(vaddr)
        .map(|(_paddr, flags)| flags != MappingFlags::empty())
        .unwrap_or(false)
}

/// 处理页表错误
pub async fn handle_page_fault(vaddr: VirtAddr, task: &MicroKernelTask) {
    if !is_mapped(vaddr) {
        task.set_fault(vaddr.addr(), PageFaultReason::USER | PageFaultReason::WRITE);
        task.handle_page_fault().await;
    }
}

// TODO: Check object cross more pages.
#[allow(dead_code)]
impl<T> UserBuffer<T> {
    #[inline]
    pub fn addr(&self) -> usize {
        self.addr.addr()
    }
    #[inline]
    pub async fn get_ref(&self, task: &MicroKernelTask) -> &'static T {
        handle_page_fault(self.addr, task).await;
        self.addr.get_ref::<T>()
    }

    #[inline]
    pub async fn get_mut(&self, task: &MicroKernelTask) -> &'static mut T {
        handle_page_fault(self.addr, task).await;
        self.addr.get_mut_ref::<T>()
    }

    #[inline]
    pub async fn slice_mut_with_len(&self, len: usize, task: &MicroKernelTask) -> &'static mut [T] {
        handle_page_fault(self.addr, task).await;
        self.addr.slice_mut_with_len(len)
    }
}

impl UserBuffer<u8> {
    #[inline]
    pub async fn slice_with_until_valid(&self, task: &MicroKernelTask) -> &'static [u8] {
        handle_page_fault(self.addr, task).await;
        let mut len = 0;
        let ptr = self.addr.addr() as *const u8;
        unsafe {
            loop {
                if *ptr.add(len) == 0 {
                    break;
                }
                len += 1;
            }
            core::slice::from_raw_parts(ptr, len)
        }
    }

    pub async fn get_str(&self, task: &MicroKernelTask) -> Result<String, Utf8Error> {
        handle_page_fault(self.addr, task).await;
        Ok(String::from_utf8_lossy(self.slice_with_until_valid(task).await).to_string())
    }
}

#[allow(dead_code)]
pub fn hexdump(data: &[u8], mut start_addr: usize) {
    const PRELAND_WIDTH: usize = 70;
    println!("{:-^1$}", " hexdump ", PRELAND_WIDTH);
    for offset in (0..data.len()).step_by(16) {
        print!("{:08x} ", start_addr);
        start_addr += 0x10;
        for i in 0..16 {
            if offset + i < data.len() {
                print!("{:02x} ", data[offset + i]);
            } else {
                print!("{:02} ", "");
            }
        }

        print!("{:>6}", ' ');

        for i in 0..16 {
            if offset + i < data.len() {
                let c = data[offset + i];
                if c >= 0x20 && c <= 0x7e {
                    print!("{}", c as char);
                } else {
                    print!(".");
                }
            } else {
                print!("{:02} ", "");
            }
        }

        println!("");
    }
    println!("{:-^1$}", " hexdump end ", PRELAND_WIDTH);
}
