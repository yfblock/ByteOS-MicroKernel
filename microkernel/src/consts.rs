/// 默认用户程序的栈大小为 20 * PAGE_SIZE = 800KB;
pub const USER_STACK_PAGES: usize = 20;

/// 默认的用户程序栈顶地址
pub const USER_STACK_TOP_ADDR: usize = 0xF000_0000;
