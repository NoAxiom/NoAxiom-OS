# NoAxiom Makefile

# general project config
export PROJECT_NAME := NoAxiom
export TARGET := riscv64gc-unknown-none-elf
export MODE ?= release
export BOARD ?= qemu-virt

# top config
PROJECT_DIR := $(PROJECT_NAME)
TARGET_DIR := target/$(TARGET)/$(MODE)
BOOTLOADER_BIN := $(PROJECT_NAME)/bootloader/rustsbi-qemu.bin

# kernel config
export KERNEL_NAME := kernel
KERNEL_DIR := $(PROJECT_DIR)/$(KERNEL_NAME)
KERNEL_ELF := $(TARGET_DIR)/$(KERNEL_NAME)
KERNEL_BIN := $(KERNEL_ELF).bin
KERNEL_ENTRY_PA := 0x80200000

# console output colors
export ERROR := "\e[31m"
export WARN := "\e[33m"
export NORMAL := "\e[32m"
export RESET := "\e[0m"

all: build run

# TODO: split qemu tags to separated tag configs
run: $(KERNEL_BIN)
	@echo -e $(NORMAL)"Running..."$(RESET)
	@qemu-system-riscv64 \
            -machine virt \
            -nographic \
            -bios $(BOOTLOADER_BIN) \
            -device loader,file=$(KERNEL_BIN),addr=$(KERNEL_ENTRY_PA)

build: vendor
	@cd $(KERNEL_DIR) && make

$(KERNEL_BIN): build
	@rust-objcopy --binary-architecture=riscv64 $(KERNEL) --strip-all -O binary $(KERNEL_BIN)

clean:
	@cargo clean

vendor: $(KERNEL_DIR)/Cargo.toml
	@echo -e $(NORMAL)"Updating vendored files..."$(RESET)
	@cargo vendor

env:
	@echo -e $(NORMAL)"Check environment..."$(RESET)
	@qemu-riscv64 --version
	@echo -e $(NORMAL)"should be "$(RESET)"qemu-riscv64 version >= 7.0.0\n"
	@riscv64-unknown-elf-gcc --version
	@echo -e $(NORMAL)"should be "$(RESET)"riscv64-unknown-elf-gcc (SiFive GCC x.x.0-20xx.xx.xx)\n"
	@rustc --version
	@echo -e $(NORMAL)"should be "$(RESET)"rustc 1.83.0-nightly\n"
	@cargo --version
	@echo -e $(NORMAL)"should be "$(RESET)"cargo 1.83.0-nightly\n"

help:
	@echo -e "Usage: make [target]"
	@echo -e "Targets:"
	@echo -e $(NORMAL)"  run:       "$(RESET)"Run the OS in QEMU"
	@echo -e $(NORMAL)"  env:       "$(RESET)"Check the environment"
	@echo -e $(NORMAL)"  build:     "$(RESET)"Build the OS"
	@echo -e $(NORMAL)"  clean:     "$(RESET)"Clean the OS"
	@echo -e $(NORMAL)"  vendor:    "$(RESET)"Vendor the dependencies"
	@echo -e $(NORMAL)"  help:      "$(RESET)"Show this help message"

.PHONY: env build clean help