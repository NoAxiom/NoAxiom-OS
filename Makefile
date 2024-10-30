OS_NAME := "NoAxiom-OS"
BOOTLOADER := ./others/bootloader/rustsbi-qemu.bin
KERNEL := target/riscv64gc-unknown-none-elf/release/$(OS_NAME)
KERNEL_BIN := target/riscv64gc-unknown-none-elf/release/$(OS_NAME).bin
KERNEL_ENTRY_PA := 0x80200000

ERROR := "\e[31m"
WARN := "\e[33m"
NORMAL := "\e[32m"
RESET := "\e[0m"

run: $(KERNEL_BIN)
	@echo -e $(NORMAL)"Running..."$(RESET)
	@qemu-system-riscv64 \
            -machine virt \
            -nographic \
            -bios $(BOOTLOADER) \
            -device loader,file=$(KERNEL_BIN),addr=$(KERNEL_ENTRY_PA)

build:
	cargo build --release

$(KERNEL_BIN): build
	@rust-objcopy --binary-architecture=riscv64 $(KERNEL) --strip-all -O binary $(KERNEL_BIN)

clean:
	cargo clean
	rm -rf tmp

env:
	@echo -e $(NORMAL)"Check environment..."$(RESET)
	@qemu-riscv64 --version
	@echo -e $(NORMAL)"should be "$(RESET)"qemu-riscv64 version >= 7.0.0\n"
	@riscv64-unknown-elf-gcc --version
	@echo -e $(NORMAL)"should be "$(RESET)"riscv64-unknown-elf-gcc (SiFive GCC x.x.0-20xx.xx.xx)\n"
	@rustc --version
	@echo -e $(NORMAL)"should be "$(RESET)"rustc 1.83.0-nightly\n"
	@cargo --version
	@echo -e $(NORMAL)"should be "$(RESET)"cargo 1.83.0-nightly\n"
	@make clean
	@tree .
	@cloc .

help:
	@echo -e "Usage: make [target]"
	@echo -e "Targets:"
	@echo -e $(NORMAL)"  run:       "$(RESET)"Run the OS in QEMU"
	@echo -e $(NORMAL)"  env:       "$(RESET)"Check the environment"
	@echo -e $(NORMAL)"  build:     "$(RESET)"Build the OS"
	@echo -e $(NORMAL)"  clean:     "$(RESET)"Clean the OS"
	@echo -e $(NORMAL)"  help:      "$(RESET)"Show this help message"

.Phony: env build clean help