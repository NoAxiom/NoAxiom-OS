# NoAxiom Makefile

# general config
export PROJECT := NoAxiom
export MODE ?= release
export KERNEL ?= kernel
export ARCH_NAME ?= riscv64
export ROOT := $(shell pwd)
export LOG ?= DEBUG
export ELF_PATH ?=   # This is for mk_fs.sh
export TEST_TYPE ?= Official
export ERROR := "\e[31m"
export WARN := "\e[33m"
export NORMAL := "\e[32m"
export RESET := "\e[0m"

ifeq ($(ARCH_NAME),riscv64)
	export TARGET := riscv64gc-unknown-none-elf
	export OBJDUMP := riscv64-unknown-elf-objdump
	export OBJCOPY := rust-objcopy --binary-architecture=riscv64
	export SBI ?= $(ROOT)/$(PROJECT)/bootloader/rustsbi-qemu.bin
	export QEMU := qemu-system-riscv64
	export MULTICORE := 2
else ifeq ($(ARCH_NAME),loongarch64)
	export TARGET := loongarch64-unknown-linux-gnu
	export OBJDUMP := loongarch64-linux-gnu-objdump
	export OBJCOPY := loongarch64-linux-gnu-objcopy
	export SBI ?= $(ROOT)/$(PROJECT)/bootloader/u-boot-with-spl.bin
	export QEMU := qemu-system-loongarch64
	export MULTICORE := 1
endif

export TARGET_DIR := $(ROOT)/target/$(TARGET)/$(MODE)

# kernel config
KERNEL_O_PATH := ./target/$(TARGET)/$(MODE)
KERNEL_ELF := $(KERNEL_O_PATH)/$(KERNEL)
KERNEL_BIN := $(KERNEL_ELF).bin
KERNEL_SYMBOL_TABLE := $(KERNEL_ELF).txt

TEST_DIR := $(ROOT)/$(PROJECT)-OS-Test
FS_IMG := $(TEST_DIR)/fs-$(ARCH_NAME).img
MKFS_SH := ./mk_fs.sh

QFLAGS := 
ifeq ($(ARCH_NAME),loongarch64)
	QFLAGS += -kernel $(KERNEL_ELF)
	QFLAGS += -m 1024
	QFLAGS += -nographic
	QFLAGS += -smp $(MULTICORE)
	QFLAGS += -drive file=$(FS_IMG),if=none,format=raw,id=x0
	QFLAGS += -device virtio-blk-pci,drive=x0 -no-reboot # Official says bus=virtio-mmio-bus.0, but it's wrong
	# QFLAGS += -device virtio-net-pci,netdev=net0
	QFLAGS += -netdev user,id=net0,hostfwd=tcp::5555-:5555,hostfwd=udp::5555-:5555
	QFLAGS += -rtc base=utc
	# QFLAGS += -drive file=disk-la.img,if=none,format=raw,id=x1
	# QFLAGS += -device virtio-blk-pci,drive=x1,bus=virtio-mmio-bus.1
else
	QFLAGS += -machine virt -kernel kernel-qemu
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
endif

default: build_user build_kernel run

$(FS_IMG):
	cd $(TEST_DIR) && make all

build_kernel: $(FS_IMG)
	@cd $(PROJECT)/kernel && make build

build_user:
	@cd $(PROJECT)/user && make build

asm:
	@echo -e $(NORMAL)"Building Kernel and Generating Assembly..."$(RESET)
	@cd $(PROJECT)/kernel && make asm
	@echo -e $(NORMAL)"Assembly saved to $(ROOT)/log/kernel.asm"$(RESET)

user_asm:
	@echo -e "Building User and Generating Assembly..."
	@cd $(PROJECT)/user && make asm

# backup: 
# 	@cp $(ROOTFS) $(SDCARD_BAK) 

# fs-backup: 
# 	@cp $(TESTFS) $(FS_BAK) 

run: sbi-qemu
	@cp $(KERNEL_BIN) kernel-qemu
	$(QEMU) $(QFLAGS)

# rm -f $(SDCARD_BAK)

# qemu-dtb:backup
# 	qemu-system-riscv64 $(QFLAGS) -machine dumpdtb=qemu.dtb
# 	@dtc -o qemu.dts -O dts -I dtb qemu.dtb
# 	rm -f $(ROOTFS)
# 	mv $(SDCARD_BAK) $(ROOTFS)

# KERNEL_ENTRY_PA := 0x80200000
# QEMU_ARGS := -machine virt \
# 			 -nographic \
# 			 -bios $(SBI) \
# 			 -device loader,file=$(KERNEL_BIN),addr=$(KERNEL_ENTRY_PA)

# gdb-server: build
# 	qemu-system-riscv64 $(QEMU_ARGS) -s -S

gdb-server: build_kernel
	$(QEMU) $(QFLAGS) -s -S


debug-client:
# loongarch64 does not support gdb
	@riscv64-unknown-elf-gdb -ex 'file $(KERNEL_BIN)' -ex 'set arch riscv:rv64' -ex 'target remote localhost:1234'

clean:
	@rm -f kernel-qemu
	@rm -f sbi-qemu
	@rm -f $(FS_IMG)
	@rm -rf $(TEST_DIR)/build
	@rm -rf $(TEST_DIR)/riscv64
	# @rm -f sdcard.img
	cargo clean

vendor:
	@cargo clean
	@cargo vendor
	@cd $(PROJECT)/user && make vendor

count:
	@echo "kernel code statistic:"
	@cd $(PROJECT) && cloc $(KERNEL) lib --exclude-dir=.trash,.trashbin --exclude-ext=md,toml

DOCKER ?= docker.educg.net/cg/os-contest:20250226

docker:
	docker run --rm -it -v .:/code --entrypoint bash -w /code --privileged $(DOCKER)

info:
	@echo "TARGET: $(TARGET)"
	@echo "ARCH_NAME: $(ARCH_NAME)"
	@echo "MODE: $(MODE)"
	@echo "MULTICORE: $(MULTICORE)"
	@echo "SBI: $(SBI)"
	@echo "QEMU: $(QEMU)"
	@echo "TEST_DIR: $(TEST_DIR)"
	@echo "FS_IMG: $(FS_IMG)"
	@echo "QFLAGS: $(QFLAGS)"

add_target:
	@echo $(NORMAL)"Adding target to rustup"$(RESET)
	@rustup target add loongarch64-unknown-none
	@rustup target add riscv64gc-unknown-none-elf

env: info add_target vendor

all: env default

.PHONY: all build run debug clean debug-client sbi-qemu backup sdcard build-gui board vendor count asm test build_user env info