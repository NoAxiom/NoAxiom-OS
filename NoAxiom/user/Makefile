APPS := $(shell find apps -maxdepth 1 -mindepth 1 -type d | sed 's|apps/||')
ELF_PATH := bin

build:
	@echo -e $(NORMAL)"Building apps..."$(RESET)
	@rm -rf $(ELF_PATH)
	@mkdir -p $(ELF_PATH)
	@cd apps && $(foreach dir, $(APPS), (cd $(dir) && cargo build --target $(TARGET) --release && cp target/$(TARGET)/$(MODE)/$(dir) ../../$(ELF_PATH) && cd ..);)
	@echo -e $(NORMAL)"Apps build finished:"$(RESET)
	@$(foreach dir, $(APPS), echo -e "\t"$(NORMAL)$(dir)$(RESET);)