GNU_OBJDUMP := riscv64-unknown-linux-gnu-objdump
EMBED_OBJDUMP := riscv64-unknown-elf-objdump
GNU_ARCH := riscv64gc-unknown-linux-gnu
EMBED_ARCH := riscv64gc-unknown-none-elf
MODE := release
EMBED_TARGET_PATH := target/$(EMBED_ARCH)/$(MODE)
GNU_TARGET_PATH := target/$(GNU_ARCH)/$(MODE)
# 共享文件子目录
LIBS_SRC := $(filter-out ./basic/src, $(wildcard ./*/src))
LIBS := $(foreach src, $(LIBS_SRC), $(patsubst ./%/src, ./%, $(src)))
# target 下共享目标文件
SO := $(wildcard $(GNU_TARGET_PATH)/*.so)
# 内核加载文件目录路径
OUPUT_PATH := ../user/target/riscv64gc-unknown-none-elf/$(MODE)
BASIC := basic

build_basic:
	cd $(BASIC) && cargo build --features inner --$(MODE);
	cp $(EMBED_TARGET_PATH)/sharedscheduler $(OUPUT_PATH)/sharedscheduler

build: build_basic
	@for x in $(LIBS); do \
	 cd $$x && cargo build --$(MODE); \
	done
	@$(foreach so, $(SO), cp $(so) $(patsubst $(GNU_TARGET_PATH)/%.so, $(OUPUT_PATH)/%, $(so));)

disasm_basic:
	$(EMBED_OBJDUMP) -S $(EMBED_TARGET_PATH)/sharedscheduler > $(EMBED_TARGET_PATH)/sharedscheduler.asm

disasm: disasm_basic
	@echo $(SO)
	$(foreach so, $(SO), $(GNU_OBJDUMP) -S $(so) > $(patsubst $(GNU_TARGET_PATH)/%.so, $(GNU_TARGET_PATH)/%.asm, $(so));)

clean:
	@rm -rf target

