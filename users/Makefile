TARGET := riscv64gc-unknown-none-elf
TARGET_ELF_DIR = target/$(TARGET)/release/
BUILD_ARGS :=

ifeq ($(TARGET), loongarch64-unknown-none)
	BUILD_ARGS += -Z build-std=core,alloc
endif

all: vm

apps: 
	cargo build --release $(BUILD_ARGS) --target $(TARGET)

vm: apps
	cd apps/vm && cargo build --release $(BUILD_ARGS) --target $(TARGET)

.PHONY: all vm apps

