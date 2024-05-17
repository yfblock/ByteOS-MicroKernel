TARGET := riscv64gc-unknown-none-elf
TARGET_ELF_DIR = target/$(TARGET)/release/

all: vm

apps: 
	cargo build --release --target $(TARGET)

vm: apps
	cd apps/vm && cargo build --release --target $(TARGET)

.PHONY: all vm apps

