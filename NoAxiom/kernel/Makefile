KERNEL_ELF := ../../target/$(TARGET)/$(MODE)/$(KERNEL)
KERNEL_BIN := $(KERNEL_ELF).bin

ifeq ($(MODE),release)
    CARGO_ARGS += --release
endif

FEATURES :=

VF2 :=
QEMU := 1
ifeq ($(ARCH_NAME),riscv64)
	INTERRUPTABLE_ASYNC := 1
else ifeq ($(ARCH_NAME),loongarch64)
	ASYNC := 1
endif
GLIBC := 1
UART := 
PRELIMINARY :=
BUSYBOX :=
RUNTESTS :=
TIME_TRACER := 
UITEST :=
DEV := 
SH := 
RAMDISK :=

ifneq ($(MULTICORE), 1)
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

ifneq ($(QEMU), )
	FEATURES += qemu
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

ifneq ($(GLIBC), )
	FEATURES += glibc
endif

ifneq ($(INTERRUPTABLE_ASYNC), )
	FEATURES += interruptable_async
endif

ifneq ($(ASYNC), )
	FEATURES += async
endif

ifneq ($(FEATURES), )
    CARGO_ARGS += --features "$(FEATURES)"
endif

all: kernel

kernel:
	@echo -e $(NORMAL)"Building Kernel..."$(RESET)
	cargo build $(CARGO_ARGS) --target $(TARGET)
	$(OBJCOPY) $(KERNEL_ELF) --strip-all -O binary $(KERNEL_BIN)

# TODO: add initproc dependency when impl user
build: kernel
	@echo -e $(NORMAL)"Kernel Build Finished."$(RESET)

initproc:
	@cd ../user/apps && make build

asm:
	@cargo objdump $(CARGO_ARGS) --target $(TARGET) --quiet -- -d > $(ROOT)/log/kernel.asm

.PHONY: kernel build initproc asm