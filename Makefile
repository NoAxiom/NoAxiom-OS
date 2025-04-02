# NoAxiom Makefile

# general config
export PROJECT := NoAxiom
export MODE ?= release
export KERNEL ?= kernel
export ARCH_NAME ?= loongarch64

export ROOT := $(shell pwd)

ifeq ($(ARCH_NAME),riscv64)
	export TARGET := riscv64gc-unknown-none-elf
	export OBJDUMP := riscv64-unknown-elf-objdump
	export OBJCOPY := rust-objcopy --binary-architecture=riscv64
	export SBI ?= $(ROOT)/$(PROJECT)/bootloader/rustsbi-qemu.bin
	export QEMU := qemu-system-riscv64
else ifeq ($(ARCH_NAME),loongarch64)
	export TARGET := loongarch64-unknown-linux-gnu
	export OBJDUMP := loongarch64-linux-gnu-objdump
	export OBJCOPY := loongarch64-linux-gnu-objcopy
	export SBI ?= $(ROOT)/$(PROJECT)/bootloader/u-boot-with-spl.bin
	export QEMU := qemu-system-loongarch64
endif

export TARGET_DIR := $(ROOT)/target/$(TARGET)/$(MODE)

export LOG ?= DEBUG

# choose Custom or Official test samples
SAMPLE := Official

# now is EXT4 fs, you can change to FAT32 in the mk_fs.sh

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
KERNEL_SYMBOL_TABLE := $(KERNEL_ELF).txt

# TFTPBOOT := /work/tftpboot/

# SDCARD_BAK = $(ROOTFS).bak
# FS_BAK = $(TESTFS).bak

TEST_DIR := ./test/riscv-syscalls-testing/user
FS_IMG := fs.img
MKFS_SH := ./mk_fs.sh


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

test:
	@rm -f $(TEST_DIR)/src/oscomp/init_proc.c
	@rm -f $(TEST_DIR)/src/oscomp/test_points.h
	@rm -f $(TEST_DIR)/src/oscomp/init_proc.h
	@cp $(PROJECT)/init_proc/init_proc.c $(TEST_DIR)/src/oscomp/init_proc.c
	@cp $(PROJECT)/init_proc/init_proc.h $(TEST_DIR)/src/oscomp/init_proc.h
	@cp $(PROJECT)/init_proc/test_points.h $(TEST_DIR)/src/oscomp/test_points.h
	@cd $(TEST_DIR) && make clean
	-@cd $(TEST_DIR) && make $(TEST_FLAGS)
	$(MKFS_SH);

build_kernel:
	@if [ $(PROJECT)/init_proc/init_proc.c -nt $(TEST_DIR)/src/oscomp/init_proc.c ] \
	|| [ $(PROJECT)/init_proc/init_proc.h -nt $(TEST_DIR)/src/oscomp/init_proc.h ] \
	|| [ $(PROJECT)/init_proc/test_points.h -nt $(TEST_DIR)/src/oscomp/test_points.h ]; then \
		make test; \
	fi
	@cd $(PROJECT)/kernel && make build

build: $(FS_IMG) build_kernel

asm: # build_kernel
	@echo -e "Building Kernel and Generating Assembly..."
	@$(OBJDUMP) -d $(KERNEL_ELF) > $(KERNEL_ELF).asm
	@echo -e "Assembly saved to $(KERNEL_ELF).asm"

# NOTE THAT if you want to run in single core
# you should export this as empty
export MULTICORE := 2

QFLAGS := 
ifeq ($(ARCH_NAME),loongarch64)
	QFLAGS += -kernel $(KERNEL_ELF)
	QFLAGS += -m 1024
	QFLAGS += -nographic
	QFLAGS += -smp $(MULTICORE)
	QFLAGS += -drive file=$(FS_IMG),if=none,format=raw,id=x0
	QFLAGS += -device virtio-blk-pci,drive=x0 -no-reboot # Official says bus=virtio-mmio-bus.0, but it's wrong
	# QFLAGS += -device virtio-net-pci,netdev=net0
	# QFLAGS += -netdev user,id=net0,hostfwd=tcp::5555-:5555,hostfwd=udp::5555-:5555
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
	QFLAGS += -drive file=fs2.img,if=none,format=raw,id=x1
	QFLAGS += -device virtio-blk-pci,drive=x1
	QFLAGS += -no-reboot -device virtio-net-device,netdev=net -netdev user,id=net
	QFLAGS += -rtc base=utc
	# QFLAGS += -drive file=disk.img,if=none,format=raw,id=x1 
	# QFLAGS += -device virtio-blk-device,drive=x1,bus=virtio-mmio-bus.1
endif

# backup: 
# 	@cp $(ROOTFS) $(SDCARD_BAK) 

# fs-backup: 
# 	@cp $(TESTFS) $(FS_BAK) 

run: sbi-qemu
	@cp $(KERNEL_BIN) kernel-qemu
	make asm
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
	cargo clean
	cargo vendor

count:
	@echo "kernel code statistic:"
	@cd $(PROJECT) && cloc $(KERNEL) lib --exclude-dir=.trash,.trashbin --exclude-ext=md,toml

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