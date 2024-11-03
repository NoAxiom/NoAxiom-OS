KERNEL_ELF := ../../target/$(TARGET)/$(MODE)/$(KERNEL_NAME)
KERNEL_BIN := $(KERNEL_ELF).bin

CARGO_ARGS :=

ifeq ($(MODE),release)
    CARGO_ARGS += --release
endif

FEATURES :=

# feature flags
MULTICORE := 
VF2 :=
UART := 
QEMU_RISCV :=
PRELIMINARY :=
BUSYBOX :=
RUNTESTS :=
TIME_TRACER := 
UITEST :=
DEV := 
SH := 
RAMDISK :=

ifneq ($(MULTICORE), )
	FEATURES += multicore
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

ifneq ($(QEMU_RISCV), )
	FEATURES += qemu_riscv
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

ifneq ($(FEATURES), )
    CARGO_ARGS += --features "$(FEATURES)"
endif

all: kernel

kernel:
	@echo -e $(NORMAL)"Building Kernel..."$(RESET)
	@cargo build $(CARGO_ARGS)
	@$(OBJCOPY) $(KERNEL_ELF) --strip-all -O binary $(KERNEL_BIN)

build: initproc kernel
	@echo -e $(NORMAL)"Build Finish."$(RESET)

initproc:
	@cd ../user/apps && make build
	
.PHONY: kernel build initproc