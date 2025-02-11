# NoAxiom Makefile

# general config
export PROJECT := NoAxiom
export TARGET := riscv64gc-unknown-none-elf
export MODE ?= release
export BOARD ?= qemu-virt
export KERNEL ?= kernel

export ROOT := $(shell pwd)
export TARGET_DIR := $(ROOT)/target/$(TARGET)/$(MODE)
export SBI ?= $(ROOT)/$(PROJECT)/bootloader/rustsbi-qemu.bin

export LOG ?= DEBUG

# choose Custom or Official test samples
SAMPLE := Official
MKFS_SH := mk_fat32img.sh

CHOSEN_PATN := 

ifeq ($(SAMPLE), Custom)
	CHOSEN_PATN := ./target/riscv64gc-unknown-none-elf/release
else ifeq ($(SAMPLE), Official)
	CHOSEN_PATN := ./test/riscv-syscalls-testing/user/build/riscv64
else 
	CHOSEN_PATN := Please check your ELF path.
endif

export ELF_PATH ?= $(CHOSEN_PATN)

# partition config
# export ROOTFS  ?= $(ROOT)/part/img/sdcard-riscv.img
# export TESTFS  ?= $(ROOT)/fs.img


# kernel config
KERNEL_O_PATH := ./target/$(TARGET)/$(MODE)
KERNEL_ELF := $(KERNEL_O_PATH)/$(KERNEL)
KERNEL_BIN := $(KERNEL_ELF).bin

# TFTPBOOT := /work/tftpboot/

# SDCARD_BAK = $(ROOTFS).bak
# FS_BAK = $(TESTFS).bak

TEST_DIR := ./test/riscv-syscalls-testing/user
FS_IMG := fs.img
MKFS_SH := ./mk_fat32img.sh

export OBJCOPY := rust-objcopy --binary-architecture=riscv64


# console output colors
export ERROR := "\e[31m"
export WARN := "\e[33m"
export NORMAL := "\e[32m"
export RESET := "\e[0m"

all: build_kernel run
	@cp $(KERNEL_BIN) kernel-qemu

build_user:
	@cd $(PROJECT)/user && make build

$(FS_IMG): build_user
	@$(MKFS_SH)

TEST_FLAGS :=
TEST_FLAGS += all
TEST_FLAGS += CHAPTER=7

$(TEST_DIR)/build: 
	@cd $(TEST_DIR) && make $(TEST_FLAGS)

test: $(TEST_DIR)/build
	@$(MKFS_SH)

build_kernel:
	@cd $(PROJECT)/kernel && make build

build: $(FS_IMG) build_kernel

asm: # build_kernel
	@echo -e "Building Kernel and Generating Assembly..."
	@riscv64-unknown-elf-objdump -d $(KERNEL_ELF) > $(KERNEL_ELF).asm
	@echo -e "Assembly saved to $(KERNEL_ELF).asm"

# NOTE THAT if you want to run in single core
# you should export this as empty
export MULTICORE_ARGS := 2

QFLAGS := 
QFLAGS += -m 128
QFLAGS += -machine virt
QFLAGS += -nographic
QFLAGS += -kernel kernel-qemu
QFLAGS += -device loader,file=$(KERNEL_BIN),addr=0x80200000
QFLAGS += -drive file=$(FS_IMG),if=none,format=raw,id=x0
QFLAGS += -device virtio-blk-device,drive=x0,bus=virtio-mmio-bus.0 
# QFLAGS += -device virtio-net-device,netdev=net -netdev user,id=net

ifneq ($(MULTICORE_ARGS),)
	QFLAGS += -smp $(MULTICORE_ARGS)
endif

ifeq ($(BOARD), qemu-virt)
	QFLAGS += -bios sbi-qemu
endif


# backup: 
# 	@cp $(ROOTFS) $(SDCARD_BAK) 

# fs-backup: 
# 	@cp $(TESTFS) $(FS_BAK) 

sbi-qemu:
	@cp $(SBI) sbi-qemu

run: sbi-qemu
	@cp $(KERNEL_BIN) kernel-qemu
	qemu-system-riscv64 $(QFLAGS)
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
	qemu-system-riscv64 $(QFLAGS) -s -S


debug-client:
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
	cargo clean
	cargo vendor

count:
	@echo "kernel code statistic:"
	@cd $(PROJECT) && cloc $(KERNEL) --exclude-dir=.trashbin --exclude-ext=md,toml

# sdcard:
# 	@echo "\e[49;34m\e[1m----------Making sdcard-----------\e[0m"
# 	@rm -f $(FS_IMG)
# 	@dd if=/dev/zero of=$(FS_IMG) count=2048 bs=1M
# 	@sudo losetup -f $(FS_IMG)
# 	@mkfs.ext4  -F -O ^metadata_csum_seed $(FS_IMG)
# 	@mkdir -p mnt
# 	@sudo mount $(FS_IMG) mnt
# 	@sudo cp -r $(TEST_DIR)/* mnt
# 	@sudo umount mnt
# 	@sudo rm -rf mnt
# 	@sudo chmod 777 $(FS_IMG)
# 	@echo "\e[49;34m\e[1m----------Making sdcard finished-----------\e[0m"


# build-gui:
# 	@echo "\e[49;34m\e[1m----------Building user-----------\e[0m"
# 	@cd $(PROJECT)/user/apps && make build
# 	@echo "\e[49;34m\e[1m----------Making fs.img-----------\e[0m"
# 	@./make_fs.sh
# 	@echo "\e[49;34m\e[1m----------Building kernel---------\e[0m"
# 	@cd $(PROJECT)/kernel && make kernel

# run-uitest: sbi-qemu
# 	@echo "\e[49;34m\e[1m----------GUI Test-----------\e[0m"
# 	@cp $(KERNEL_BIN) kernel-qemu
# 	@qemu-system-riscv64 \
# 		-machine virt \
# 		-nographic \
# 		-kernel kernel-qemu \
# 		-drive file=$(TESTFS),if=none,format=raw,id=x0 \
# 		-vnc :0\
# 	        -device virtio-blk-device,drive=x0,bus=virtio-mmio-bus.0 \
# 		-device virtio-net-device,netdev=net -netdev user,id=net\
# 		-device virtio-gpu-device,xres=640,yres=480\
# 		-device virtio-mouse-device\
# 		-device virtio-keyboard-device

# gvnc:
# 	@echo "\e[49;34m\e[1m----------Open Viewer-----------\e[0m"
# 	gvncviewer localhost

# board:
# 	@cp $(TARGET_DIR)/$(KERNEL).bin  $(TFTPBOOT)

.PHONY: all build run debug clean debug-client sbi-qemu backup sdcard build-gui board vendor count asm test