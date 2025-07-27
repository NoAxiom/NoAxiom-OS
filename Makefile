# NoAxiom Makefile

# General config
include config.mk
export PROJECT := NoAxiom
export MODE := release
export KERNEL := kernel
export TEST_TYPE ?= custom
export ARCH_NAME ?= riscv64
export LIB_NAME ?= musl
export INIT_PROC ?= busybox
export ON_SCREEN ?= false
export ROOT := $(shell pwd)
export LOG ?= DEBUG
export CONFIG ?= default
export USER_PROJECT := NoAxiom-OS-User
export UTILS := NoAxiom-OS-Utils
export ERROR := "\e[31m"
export WARN := "\e[33m"
export NORMAL := "\e[32m"
export RESET := "\e[0m"
export RELEASE ?= false
export TOOLCHAIN_DIR := $(ROOT)/$(UTILS)/toolchain
export FEAT_ON_QEMU ?= true

# Arch config
ifeq ($(ARCH_NAME),riscv64) # RISC-V64
export TARGET := riscv64gc-unknown-none-elf
export OBJDUMP := riscv64-unknown-elf-objdump
export OBJCOPY := rust-objcopy --binary-architecture=riscv64
export QEMU := qemu-system-riscv64
export MULTICORE ?= 1
export GDB := $(TOOLCHAIN_DIR)/riscv64-unknown-elf-gdb
export SIMPLE_ARCH_NAME := rv
else ifeq ($(ARCH_NAME),loongarch64) # LoongArch64
export TARGET := loongarch64-unknown-linux-gnu
export OBJDUMP := loongarch64-linux-gnu-objdump
export OBJCOPY := loongarch64-linux-gnu-objcopy
export QEMU := qemu-system-loongarch64
export MULTICORE ?= 1
export GDB := $(TOOLCHAIN_DIR)/loongarch64-linux-gnu-gdb
export SIMPLE_ARCH_NAME := la
endif

# Kernel config
export TARGET_DIR := $(ROOT)/$(PROJECT)/target/$(TARGET)/$(MODE)
export KERNEL_ELF := $(TARGET_DIR)/$(KERNEL)
export KERNEL_BIN := $(KERNEL_ELF).bin
export KERNEL_SYMBOL_TABLE := $(KERNEL_ELF).txt
ifeq ($(ARCH_NAME),riscv64) # RISC-V64
export KERNEL_FILE := $(KERNEL_BIN)
else ifeq ($(ARCH_NAME),loongarch64) # LoongArch64
export KERNEL_FILE := $(KERNEL_ELF)
endif

# Test and fs image config
TEST_DIR := $(ROOT)/$(PROJECT)-OS-Test
RAW_FS_IMG := $(TEST_DIR)/$(TEST_TYPE)/img/fs-$(ARCH_NAME).img
FS_IMG_DIR := $(TEST_DIR)/$(TEST_TYPE)/tmp-img
FS_IMG := $(TEST_DIR)/$(TEST_TYPE)/tmp-img/fs-$(ARCH_NAME).fs.img

# Qemu flags config
QFLAGS := 
ifeq ($(ARCH_NAME),riscv64) # RISC-V64
QFLAGS += -machine virt -kernel $(KERNEL_FILE)
QFLAGS += -m 1G
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
QFLAGS += -kernel $(KERNEL_FILE)
QFLAGS += -m 1G
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

MAKE_OPTION ?= build run
default: $(MAKE_OPTION)

CONFIG_DIR := ./$(UTILS)/config
CONFIG_FILE := $(CONFIG_DIR)/$(CONFIG).mk
config.mk:
	@touch ./config.mk

config:
	@echo "using config: $(CONFIG)"
	@cp $(CONFIG_DIR)/$(CONFIG).mk $(ROOT)/config.mk

build: build-user build-kernel
	@cp $(KERNEL_FILE) ./kernel-$(SIMPLE_ARCH_NAME)

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
run: backup
	@cp $(KERNEL_BIN) kernel-qemu
	@echo -e $(NORMAL)"Qemu launched. Log is saved to: $(LOG_SAVE_PATH)"$(RESET)
	@$(QEMU) $(QFLAGS) $(RUN_OPTION)
	@echo -e $(NORMAL)"Qemu exited. Log is saved to: $(LOG_SAVE_PATH)"$(RESET)


TEST_2K1000_DIR := $(ROOT)/$(UTILS)/la-2k1000-sim
test_2k1000:
	@echo -e $(NORMAL)"Running 2k-1000 tests..."$(RESET)
	@echo -e $(NORMAL)"Checking la-2k1000-sim submodule..."$(RESET)
	@cd $(ROOT)/$(UTILS) && \
	if [ -d "la-2k1000-sim/.git" ] || [ -f "la-2k1000-sim/.git" ]; then \
		echo -e $(NORMAL)"Submodule exists, updating to latest..."$(RESET); \
		cd la-2k1000-sim && git fetch origin && git pull origin main; \
	else \
		echo -e $(NORMAL)"Submodule not found, initializing and updating..."$(RESET); \
		git submodule update --init la-2k1000-sim; \
	fi
	@cd $(TEST_2K1000_DIR) && make

QEMU_DTB = log/qemu-$(ARCH_NAME).dtb
QEMU_DTS = log/qemu-$(ARCH_NAME).dts
qemu-dumpdtb:
	@rm -f $(QEMU_DTS) $(QEMU_DTB)
	$(QEMU) $(QFLAGS) -machine dumpdtb=$(QEMU_DTB)
	@dtc -o $(QEMU_DTS) -O dts -I dtb $(QEMU_DTB)
	@echo "QEMU DTS files have been generated in $(QEMU_DTS)"

gdb-server: backup build
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
	@rm -f ./kernel-rv
	@rm -f ./kernel-la
	@cd $(PROJECT) && cargo clean

vendor:
	@cd $(PROJECT) && cargo clean
	@cd $(PROJECT) && cargo vendor
	@cd $(USER_PROJECT) && make vendor

count:
	@echo "kernel code statistic:"
	@cd $(PROJECT) && cloc $(KERNEL) lib --exclude-dir=.trash,.trashbin --exclude-ext=md,toml

DOCKER ?= docker.educg.net/cg/os-contest:20250605
docker:
	docker run --rm -it -v .:/code --entrypoint bash -w /code --privileged $(DOCKER)

info:
	@echo "TARGET: $(TARGET)"
	@echo "ARCH_NAME: $(ARCH_NAME)"
	@echo "LIB_NAME: $(LIB_NAME)"
	@echo "INIT_PROC: $(INIT_PROC)"
	@echo "MODE: $(MODE)"
	@echo "MULTICORE: $(MULTICORE)"
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
	@git submodule update --init --recursive
	@echo "Submodules updated."
	@echo "Checking submodule status..."
	@git submodule status

vscode:
	@echo "Copying vscode template..."
	@mkdir -p $(ROOT)/.vscode
	@cp -f $(ROOT)/$(UTILS)/vscode-template/* $(ROOT)/.vscode/
	@echo "VSCode template copied."

switch-arch:
	@echo "Switching architecture..."
	@cd $(UTILS)/scripts && sh vscode_switch_arch.sh

env: add-target git-update vendor

build-all:
	@make build ARCH_NAME=riscv64 LOG=OFF RELEASE=true INIT_PROC=runtests
	@make build ARCH_NAME=loongarch64 LOG=OFF RELEASE=true INIT_PROC=runtests

all: build-all
	@echo "Kernel build finished. See output elf in kernel-rv & kernel-la"

.PHONY: default all build run clean      # basic make
.PHONY: gdb-server gdb                   # debug client
.PHONY: asm asm-user asm-all             # generate assembly info
.PHONY: build-user build-kernel          # for more specific build
.PHONY: add-target env git-update vendor # environment setup
.PHONY: config vscode switch-arch        # config
.PHONY: info help count                  # information utils
.PHONY: docker build-all                 # for competition