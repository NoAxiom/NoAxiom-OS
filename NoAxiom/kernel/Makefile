ifeq ($(MODE),release)
CARGO_ARGS += --release
endif

FEATURES :=

QEMU := 1
ifeq ($(ARCH_NAME),riscv64)
FULL_FUNC := 1
else ifeq ($(ARCH_NAME),loongarch64)
ASYNC := 1
endif
LOG_PRINT := 

ifneq ($(RELEASE),true)
DEBUG_SIG := 1
endif

ifneq ($(MULTICORE), 1)
	FEATURES += multicore
endif

ifneq ($(QEMU), )
	FEATURES += qemu
endif

ifeq ($(INIT_PROC),busybox)
	FEATURES += busybox
else ifeq ($(INIT_PROC),runtests)
	FEATURES += runtests
else
	$(error "Invalid INIT_PROC value: $(INIT_PROC)")
endif

ifneq ($(FULL_FUNC), )
	FEATURES += full_func
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