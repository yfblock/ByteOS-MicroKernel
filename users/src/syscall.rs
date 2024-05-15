use core::arch::asm;

use syscall_consts::SysCall;

#[cfg(target_arch = "riscv64")]
fn syscall(id: usize, args: [usize; 3]) -> isize {
    let mut ret: isize;
    unsafe {
        asm!(
            "ecall",
            inlateout("x10") args[0] => ret,
            in("x11") args[1],
            in("x12") args[2],
            in("x17") id
        );
    }
    ret
}

#[cfg(target_arch = "aarch64")]
fn syscall(id: usize, args: [usize; 3]) -> isize {
    let mut ret: isize;
    unsafe {
        asm!(
            "svc #0",
            inlateout("x0") args[0] => ret,
            in("x1") args[1],
            in("x2") args[2],
            in("x8") id
        );
    }
    ret
}

#[cfg(target_arch = "x86_64")]
fn syscall(id: usize, args: [usize; 3]) -> isize {
    let mut ret: isize;
    unsafe {
        asm!(
            "
                push r11
                push rcx
                syscall
                pop  rcx
                pop  r11
            ",
            in("rdi") args[0],
            in("rsi") args[1],
            in("rdx") args[2],
            inlateout("rax") id => ret
        );
    }
    ret
}

#[cfg(target_arch = "loongarch64")]
fn syscall(id: usize, args: [usize; 3]) -> isize {
    let mut ret: isize;
    unsafe {
        asm!(
            "syscall 0",
            inlateout("$r4") args[0] => ret,
            in("$r5") args[1],
            in("$r6") args[2],
            in("$r11") id
        );
    }
    ret
}

pub fn serial_write(buf: &[u8]) -> usize {
    syscall(SysCall::SerialWrite.into(), [buf.as_ptr() as usize, buf.len(), 0]) as _
}

pub fn sleep(ms: usize) -> usize {
    syscall(SysCall::Time.into(), [ms, 0, 0]) as _
}

pub fn exit() -> ! {
    syscall(SysCall::TaskExit.into(), [0, 0, 0]);
    unreachable!("This task should already exited.")
}
