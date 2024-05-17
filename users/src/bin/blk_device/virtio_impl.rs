use alloc::vec::Vec;
use core::ptr::NonNull;
use spin::{Lazy, Mutex};
use users::{
    syscall::{alloc_memory, translate_vaddr},
    BLOCK_SIZE,
};
use virtio_drivers::{BufferDirection, Hal, PhysAddr, PAGE_SIZE};

/// 保存内存区域映射关系，(vaddr, paddr)
pub static MEMORY: Mutex<Vec<(usize, usize)>> = Mutex::new(Vec::new());

/// 保存申请到的 BUFFER, 这个 Buffer 可以很方便进行页表映射
static BUFFER: Lazy<(usize, usize)> = Lazy::new(|| {
    let addr = alloc_memory(PAGE_SIZE).expect("can't allocate memory");
    println!("alloc memory {:#x?}", addr);
    MEMORY.lock().push(addr);
    addr
});

pub struct HalImpl;

unsafe impl Hal for HalImpl {
    fn dma_alloc(pages: usize, _direction: BufferDirection) -> (PhysAddr, NonNull<u8>) {
        let (vaddr, paddr) = alloc_memory(pages * PAGE_SIZE).expect("can't alloc memory");
        MEMORY.lock().push((vaddr, paddr));
        (
            paddr,
            NonNull::new(vaddr as _).expect("Nonnull shouldn't be null"),
        )
    }

    unsafe fn dma_dealloc(paddr: PhysAddr, _vaddr: NonNull<u8>, pages: usize) -> i32 {
        // nothing to do
        0
    }

    unsafe fn mmio_phys_to_virt(paddr: PhysAddr, _size: usize) -> NonNull<u8> {
        println!("translate physical {:#x}", paddr);
        NonNull::new(paddr as _).unwrap()
    }

    unsafe fn share(buffer: NonNull<[u8]>, _direction: BufferDirection) -> PhysAddr {
        let vaddr = buffer.as_ptr() as *mut u8 as usize;
        println!("share {:#x}", vaddr);
        let target_address = translate_vaddr(vaddr).unwrap_or(0);
        println!("target address {:#x}", target_address);
        target_address
    }

    unsafe fn unshare(_paddr: PhysAddr, _buffer: NonNull<[u8]>, _direction: BufferDirection) {
        // Nothing to do, as the host already has access to all memory and we didn't copy the buffer
        // anywhere else.
    }
}

impl HalImpl {
    /// 获取 read 使用的 buffer
    pub fn read_buffer() -> &'static [u8] {
        unsafe { core::slice::from_raw_parts(BUFFER.0 as *const u8, BLOCK_SIZE) }
    }
    /// 获取 write 使用的 buffer
    pub fn write_buffer() -> &'static mut [u8] {
        unsafe { core::slice::from_raw_parts_mut((BUFFER.0 + BLOCK_SIZE) as *mut u8, BLOCK_SIZE) }
    }
}
