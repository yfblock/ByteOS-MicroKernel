#![feature(lazy_cell)]

#[allow(unused_macros)]
macro_rules! display {
    ($fmt:expr) => (println!("cargo:warning={}", format!($fmt)));
    ($fmt:expr, $($arg:tt)*) => (println!(concat!("cargo:warning=", $fmt), $($arg)*));
}

fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=target/riscv64gc-unknown-none-elf/release/shell");
    println!("cargo:rerun-if-changed=target/riscv64gc-unknown-none-elf/release/pong");
}
