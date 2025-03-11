KERNEL_ELF := ../../target/$(TARGET)/$(MODE)/$(KERNEL)
KERNEL_BIN := $(KERNEL_ELF).bin

CARGO_ARGS :=

ifeq ($(MODE),release)
    CARGO_ARGS += --release
endif

FEATURES :=

# feature flags
# arch, multicore config is passed from ../makefile
SV39 := 1
VF2 :=
RISCV_QEMU := 1
ASYNC_FS := 1
UART := 
PRELIMINARY :=
BUSYBOX :=
RUNTESTS :=
TIME_TRACER := 
UITEST :=
DEV := 
SH := 
RAMDISK :=

ifneq ($(MULTICORE_ARGS), )
	FEATURES += multicore
endif

ifneq ($(SV39), )
	FEATURES += sv39
endif

ifneq ($(UART), )
	FEATURES += uart
endif

ifneq ($(RAMDISK), )
	FEATURES += ramdisk
endif

ifneq ($(UITEST), )
	FEATURES += uitest
endif

ifneq ($(TIME_TRACER), )
	FEATURES += time_tracer
endif

ifneq ($(SH), )
	FEATURES += customized-sh
endif

ifneq ($(DEV), )
	FEATURES += dev
endif

ifneq ($(VF2), )
	FEATURES += vf2
endif

ifneq ($(RISCV_QEMU), )
	FEATURES += riscv_qemu
endif

ifneq ($(BUSYBOX), )
	FEATURES += busybox
endif

ifneq ($(PRELIMINARY), )
	FEATURES += preliminary
endif

ifneq ($(RUNTESTS), )
	FEATURES += runtests
endif

ifneq ($(ASYNC_FS), )
	FEATURES += async_fs
endif

ifneq ($(FEATURES), )
    CARGO_ARGS += --features "$(FEATURES)"
endif

all: kernel

kernel:
	@echo -e $(NORMAL)"Building Kernel..."$(RESET)
	cargo build $(CARGO_ARGS)
	$(OBJCOPY) $(KERNEL_ELF) --strip-all -O binary $(KERNEL_BIN)

# TODO: add initproc dependency when impl user
build: kernel
	@echo -e $(NORMAL)"Kernel Build Finished."$(RESET)

initproc:
	@cd ../user/apps && make build
	
.PHONY: kernel build initproc