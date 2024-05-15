TARGET := riscv64gc-unknown-none-elf
TARGET_ELF_DIR = target/$(TARGET)/release/

all:
	cargo build --release --target $(TARGET)
	ls $(TARGET_ELF_DIR)
