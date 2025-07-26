ifeq ($(MODE),release)
CARGO_ARGS += --release
endif

FEATURES :=

FEAT_ON_QEMU ?= true
LOG_PRINT := 

ifneq ($(RELEASE),true)
DEBUG_SIG := 1
endif

ifneq ($(MULTICORE), 1)
	FEATURES += multicore
endif

ifeq ($(FEAT_ON_QEMU),true)
	FEATURES += qemu
endif

ifeq ($(INIT_PROC),busybox)
	FEATURES += busybox
else ifeq ($(INIT_PROC),runtests)
	FEATURES += runtests
else
	$(error "Invalid INIT_PROC value: $(INIT_PROC)")
endif

ifneq ($(LOG_PRINT), )
	FEATURES += log_print
endif

ifneq ($(DEBUG_SIG), )
	FEATURES += debug_sig
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