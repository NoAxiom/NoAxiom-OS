APPS := $(shell find apps -maxdepth 1 -mindepth 1 -type d | sed 's|apps/||')
ELF_PATH := bin
TARGET_DIR := $(shell pwd)/target/$(TARGET)/$(MODE)

all: build

build:
	@echo -e $(NORMAL)"Building apps..."$(RESET)
	@rm -rf $(ELF_PATH)
	@mkdir -p $(ELF_PATH)
	@cd apps && $(foreach dir, $(APPS), (cd $(dir) && cargo build --target $(TARGET) --release && cd ..);)
	@echo -e $(NORMAL)"Apps build finished:"$(RESET)
	@$(foreach dir, $(APPS), echo -e "\t"$(NORMAL)$(dir)$(RESET);)
	@$(foreach dir, $(APPS), cp $(TARGET_DIR)/$(dir) $(ELF_PATH);)

vendor:
	@cargo clean
	@cargo vendor

asm:
	@echo -e $(NORMAL)"Generating User Assembly..."$(RESET)
	@$(foreach dir, $(APPS), (cd apps/$(dir) && cargo objdump --target $(TARGET) --release -- -d > $(ROOT)/log/$(dir).asm);)
	@echo -e $(NORMAL)"Assembly saved to:"$(RESET)
	@$(foreach dir, $(APPS), (echo -e "\t"$(ROOT)/log/$(dir).asm);)

.PHONY: all build vendor
