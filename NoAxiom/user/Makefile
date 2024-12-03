APPS := $(shell find apps -maxdepth 1 -mindepth 1 -type d | sed 's|apps/||')
ELFS := elfs

build:
	@echo -e $(NORMAL)"Building apps..."$(RESET)
	@cd apps && $(foreach dir, $(APPS), (cd $(dir) && cargo build --release && cd ..);)
	@echo -e $(NORMAL)"Apps build finished:"$(RESET)
	@$(foreach dir, $(APPS), echo -e "\t"$(NORMAL)$(dir)$(RESET);)
	@mkdir -p $(ELFS)
	-$(foreach dir, $(APPS), cp $(TARGET_DIR)/$(dir) $(ELFS);)