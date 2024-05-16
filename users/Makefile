TARGET := riscv64gc-unknown-none-elf
TARGET_ELF_DIR = target/$(TARGET)/release/

all:
	cargo build --release --target $(TARGET) --bin shell
	cargo build --release --target $(TARGET) --bin vm
