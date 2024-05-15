#![feature(lazy_cell)]

use std::env;
use std::io::Result;

#[allow(unused_macros)]
macro_rules! display {
    ($fmt:expr) => (println!("cargo:warning={}", format!($fmt)));
    ($fmt:expr, $($arg:tt)*) => (println!(concat!("cargo:warning=", $fmt), $($arg)*));
}

fn main() {
    // write module configuration to OUT_PATH, then it will be included in the main.rs
    gen_linker_script(&env::var("CARGO_CFG_BOARD").expect("can't find board"))
        .expect("can't generate linker script");
    println!("cargo:rerun-if-env-changed=CARGO_CFG_TARGET_ARCH");
    // println!("cargo:rerun-if-env-changed=CARGO_CFG_KERNEL_BASE");
    println!("cargo:rerun-if-env-changed=CARGO_CFG_BOARD");
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=../users/target/riscv64gc-unknown-none-elf/release/vm");
    println!("cargo:rerun-if-changed=linker.lds.S");
}

fn gen_linker_script(platform: &str) -> Result<()> {
    let arch = env::var("CARGO_CFG_TARGET_ARCH").expect("can't find target");
    let board = env::var("CARGO_CFG_BOARD").unwrap_or("qemu".to_string());
    let fname = format!("linker_{}_{}.lds", arch, platform);
    let (output_arch, kernel_base) = if arch == "x86_64" {
        ("i386:x86-64", "0xffffff8000200000")
    } else if arch.contains("riscv64") {
        ("riscv", "0xffffffc080200000") // OUTPUT_ARCH of both riscv32/riscv64 is "riscv"
    } else if arch.contains("aarch64") {
        ("aarch64", "0xffffff8040080000")
    } else if arch.contains("loongarch64") {
        match board.as_str() {
            "2k1000" => ("loongarch64", "0x9000000098000000"),
            _ => ("loongarch64", "0x9000000090000000"),
        }
    } else {
        (arch.as_str(), "0")
    };
    let ld_content = std::fs::read_to_string("linker.lds.S")?
        .replace("%ARCH%", output_arch)
        .replace("%KERNEL_BASE%", kernel_base)
        .replace("%SMP%", "4");

    std::fs::write(&fname, ld_content)?;
    println!("cargo:rustc-link-arg=-Tmicrokernel/{}", fname);
    Ok(())
}
