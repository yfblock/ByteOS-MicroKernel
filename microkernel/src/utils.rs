use core::marker::PhantomData;

use polyhal::addr::VirtAddr;

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

#[allow(dead_code)]
impl<T> UserBuffer<T> {
    #[inline]
    pub fn addr(&self) -> usize {
        self.addr.addr()
    }
    #[inline]
    pub fn get_ref(&self) -> &'static T {
        self.addr.get_ref::<T>()
    }

    #[inline]
    pub fn get_mut(&self) -> &'static mut T {
        self.addr.get_mut_ref::<T>()
    }

    #[inline]
    pub fn slice_mut_with_len(&self, len: usize) -> &'static mut [T] {
        self.addr.slice_mut_with_len(len)
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
