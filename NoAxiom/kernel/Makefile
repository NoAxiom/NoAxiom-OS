KERNEL_ELF := ../../target/$(TARGET)/$(MODE)/$(KERNEL)
KERNEL_BIN := $(KERNEL_ELF).bin

ifeq ($(MODE),release)
    CARGO_ARGS += --release
endif

FEATURES :=

QEMU := 1
ifeq ($(ARCH_NAME),riscv64)
	INTERRUPTABLE_ASYNC := 1
else ifeq ($(ARCH_NAME),loongarch64)
	ASYNC := 1
endif
LOG_PRINT := 
DEBUG_SIG := 1

ifneq ($(MULTICORE), 1)
	FEATURES += multicore
endif

ifneq ($(QEMU), )
	FEATURES += qemu
endif

ifeq ($(INIT_PROC),busybox)
	FEATURES += busybox
endif

ifeq ($(LIB_NAME),glibc)
	FEATURES += glibc
else ifeq ($(LIB_NAME),musl)
	FEATURES += musl
endif

ifneq ($(INTERRUPTABLE_ASYNC), )
	FEATURES += interruptable_async
endif

ifneq ($(LOG_PRINT), )
	FEATURES += log_print
endif

ifneq ($(DEBUG_SIG), )
	FEATURES += debug_sig
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

asm:
	@cargo objdump $(CARGO_ARGS) --target $(TARGET) --quiet -- -d > $(ROOT)/log/kernel.asm

.PHONY: kernel build asm