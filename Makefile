# NoAxiom Makefile

# General config
export PROJECT := NoAxiom
export MODE := release
export KERNEL := kernel
export TEST_TYPE ?= custom
export ARCH_NAME ?= riscv64
export LIB_NAME ?= musl
export INIT_PROC ?= busybox
export ON_SCREEN ?= false
export ROOT := $(shell pwd)
export LOG ?= WARN
export USER_PROJECT := NoAxiom-OS-User
export ERROR := "\e[31m"
export WARN := "\e[33m"
export NORMAL := "\e[32m"
export RESET := "\e[0m"

# Arch config
ifeq ($(ARCH_NAME),riscv64) # RISC-V64
export TARGET := riscv64gc-unknown-none-elf
export OBJDUMP := riscv64-unknown-elf-objdump
export OBJCOPY := rust-objcopy --binary-architecture=riscv64
export QEMU := qemu-system-riscv64
export MULTICORE := 1
export GDB := riscv64-unknown-elf-gdb
export SIMPLE_ARCH_NAME := rv
else ifeq ($(ARCH_NAME),loongarch64) # LoongArch64
export TARGET := loongarch64-unknown-linux-gnu
export OBJDUMP := loongarch64-linux-gnu-objdump
export OBJCOPY := loongarch64-linux-gnu-objcopy
export QEMU := qemu-system-loongarch64
export MULTICORE := 1
export GDB := $(TOOLCHAIN_DIR)/loongarch64-linux-gnu-gdb
export SIMPLE_ARCH_NAME := la
endif

# Kernel config
export TARGET_DIR := $(ROOT)/$(PROJECT)/target/$(TARGET)/$(MODE)
export TOOLCHAIN_DIR := $(ROOT)/utils/toolchain
export OUTPUT_DIR := $(ROOT)/output
export KERNEL_ELF := $(TARGET_DIR)/$(KERNEL)
export KERNEL_BIN := $(KERNEL_ELF).bin
export KERNEL_SYMBOL_TABLE := $(KERNEL_ELF).txt

# Test and fs image config
TEST_DIR := $(ROOT)/$(PROJECT)-OS-Test
RAW_FS_IMG := $(TEST_DIR)/$(TEST_TYPE)/img/$(ARCH_NAME)-$(LIB_NAME).img
FS_IMG_DIR := $(TEST_DIR)/$(TEST_TYPE)/tmp-img
FS_IMG := $(TEST_DIR)/$(TEST_TYPE)/tmp-img/$(ARCH_NAME)-$(LIB_NAME).fs.img

# Qemu flags config
QFLAGS := 
ifeq ($(ARCH_NAME),riscv64) # RISC-V64
QFLAGS += -machine virt -kernel $(KERNEL_BIN)
QFLAGS += -m 128
QFLAGS += -nographic
QFLAGS += -smp $(MULTICORE)
QFLAGS += -bios default
QFLAGS += -drive file=$(FS_IMG),if=none,format=raw,id=x0
QFLAGS += -device virtio-blk-device,drive=x0,bus=virtio-mmio-bus.0 
QFLAGS += -no-reboot -device virtio-net-device,netdev=net -netdev user,id=net
QFLAGS += -rtc base=utc
# QFLAGS += -drive file=disk.img,if=none,format=raw,id=x1 
# QFLAGS += -device virtio-blk-device,drive=x1,bus=virtio-mmio-bus.1
else ifeq ($(ARCH_NAME),loongarch64) # LoongArch64
QFLAGS += -kernel $(KERNEL_ELF)
QFLAGS += -m 1024
QFLAGS += -nographic
QFLAGS += -smp $(MULTICORE)
QFLAGS += -drive file=$(FS_IMG),if=none,format=raw,id=x0
QFLAGS += -device virtio-blk-pci,drive=x0 -no-reboot # Official says bus=virtio-mmio-bus.0, but it's wrong
QFLAGS += -netdev user,id=net0,hostfwd=tcp::5555-:5555,hostfwd=udp::5555-:5555
QFLAGS += -rtc base=utc
# QFLAGS += -device virtio-net-pci,netdev=net0
# QFLAGS += -drive file=disk-la.img,if=none,format=raw,id=x1
# QFLAGS += -device virtio-blk-pci,drive=x1,bus=virtio-mmio-bus.1
endif

default: backup build run

build: build-user build-kernel
	@mkdir -p $(OUTPUT_DIR)
	@cp $(KERNEL_ELF) $(OUTPUT_DIR)/kernel-$(SIMPLE_ARCH_NAME)

build-kernel:
	@cd $(PROJECT)/kernel && make build

build-user:
	@cd $(USER_PROJECT) && make build

asm: info
	@echo -e $(NORMAL)"Building Kernel and Generating Assembly..."$(RESET)
	@cd $(PROJECT)/kernel && make asm
	@echo -e $(NORMAL)"Assembly saved to $(ROOT)/log/kernel.asm"$(RESET)

asm-user:
	@echo -e "Building User and Generating Assembly..."
	@cd $(USER_PROJECT) && make asm

asm-all: asm asm-user

$(RAW_FS_IMG):
	@echo -e $(NORMAL)"Building FS Image..."$(RESET)
	cd $(TEST_DIR) && make

backup: $(RAW_FS_IMG)
	@echo -e $(NORMAL)"Backing up FS Image..."$(RESET)
	@cd $(TEST_DIR) && make check

LOG_SAVE_PATH := log/$(shell date +%m_%d-%H_%M).log
RUN_OPTION := 
ifeq ($(ON_SCREEN),true)
RUN_OPTION += | tee $(LOG_SAVE_PATH)
else ifeq ($(ON_SCREEN),false)
RUN_OPTION += > $(LOG_SAVE_PATH)
endif
run:
	@cp $(KERNEL_BIN) kernel-qemu
	@echo -e $(NORMAL)"Qemu launched. Log is saved to: $(LOG_SAVE_PATH)"$(RESET)
	@$(QEMU) $(QFLAGS) $(RUN_OPTION)
	@echo -e $(NORMAL)"Qemu exited. Log is saved to: $(LOG_SAVE_PATH)"$(RESET)

gdb-server: build-kernel
	$(QEMU) $(QFLAGS) -s -S

GDB_FLAGS := 
GDB_FLAGS += -ex 'file $(KERNEL_BIN)'
GDB_FLAGS += -ex 'target remote localhost:1234'
ifeq ($(ARCH_NAME),riscv64) # RISC-V64
GDB_FLAGS += -ex 'set arch riscv:rv64'
else ifeq ($(ARCH_NAME),loongarch64) # LoongArch64
GDB_FLAGS += -ex 'set arch Loongarch64'
endif
gdb:
	@$(GDB) $(GDB_FLAGS)

clean:
	@rm -f kernel-qemu
	@rm -f $(FS_IMG)
	@rm -rf $(TEST_DIR)/build
	@rm -rf $(TEST_DIR)/riscv64
	@rm -rf $(OUTPUT_DIR)
	@cd $(PROJECT) && cargo clean

vendor:
	@cd $(PROJECT) && cargo clean
	@cd $(PROJECT) && cargo vendor
	@cd $(USER_PROJECT) && make vendor

count:
	@echo "kernel code statistic:"
	@cd $(PROJECT) && cloc $(KERNEL) lib --exclude-dir=.trash,.trashbin --exclude-ext=md,toml

DOCKER ?= docker.educg.net/cg/os-contest:20250226
docker:
	docker run --rm -it -v .:/code --entrypoint bash -w /code --privileged $(DOCKER)

info:
	@echo "TARGET: $(TARGET)"
	@echo "ARCH_NAME: $(ARCH_NAME)"
	@echo "LIB_NAME: $(LIB_NAME)"
	@echo "INIT_PROC: $(INIT_PROC)"
	@echo "MODE: $(MODE)"
	@echo "MULTICORE: $(MULTICORE)"
	@echo "SBI: $(SBI)"
	@echo "QEMU: $(QEMU)"
	@echo "TEST_DIR: $(TEST_DIR)"
	@echo "FS_IMG: $(FS_IMG)"
	@echo "QFLAGS: $(QFLAGS)"
	@echo "KERNEL_BIN: $(KERNEL_BIN)"
	@echo "KERNEL_ELF: $(KERNEL_ELF)"

help:
	@echo "NoAxiom Makefile"
	@echo "Usage: make [target] [options]"
	@echo "Targets:"
	@echo "  all: setup and build the kernel"
	@echo "  default: build and run the kernel"
	@echo "  build: build the kernel and user"
	@echo "  run: run the kernel in QEMU without build"
	@echo "  clean: clean the build files"
	@echo "  env: setup the environment and update vendor files"
	@echo "Options:"
	@echo "  ARCH_NAME: specify the architecture name (riscv64, loongarch64)"
	@echo "  LIB_NAME: specify the library name (glibc, musl)"
	@echo "  LOG: specify the log level (DEBUG, INFO, WARN, ERROR, OFF)"
	@echo "  INIT_PROC: specify the init process (busybox, ...)"

add-target:
	@echo $(NORMAL)"Adding target to rustup"$(RESET)
	@rustup target add loongarch64-unknown-linux-gnu
	@rustup target add riscv64gc-unknown-none-elf

git-update:
	@echo "Updating git submodules..."
	@git submodule init
	@git submodule update
	@echo "Submodules updated."
	@echo "Checking submodule status..."
	@git submodule status

vscode:
	@echo "Copying vscode template..."
	@mkdir -p $(ROOT)/.vscode
	@cp -f $(ROOT)/utils/vscode-template/* $(ROOT)/.vscode/
	@echo "VSCode template copied."

env: add-target git-update vendor

build-all:
	@make build ARCH_NAME=riscv64 LOG=OFF
	@make build ARCH_NAME=loongarch64 LOG=OFF

all: clean env build-all
	@echo "Kernel build finished. See output elf in $(OUTPUT_DIR)"

.PHONY: default all build run clean             # basic make
.PHONY: gdb-server gdb                          # debug client
.PHONY: asm asm-user asm-all                    # generate assembly info
.PHONY: build-user build-kernel                 # for more specific build
.PHONY: add-target env git-update vendor vscode # environment setup
.PHONY: info help count                         # information utils
.PHONY: docker build-all                        # for competition