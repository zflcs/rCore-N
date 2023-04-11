OBJDUMP := rust-objdump --arch-name=riscv64
ARCH := riscv64gc-unknown-linux-gnu
MODE := release
TARGET_PATH := target/$(ARCH)/$(MODE)
# 共享文件子目录
LIBS_SRC := $(filter-out ./basic/src, $(wildcard ./*/src))
LIBS := $(foreach src, $(LIBS_SRC), $(patsubst ./%/src, ./%, $(src)))
# target 下共享目标文件
SO := $(wildcard $(TARGET_PATH)/*.so)
# 内核加载文件目录路径
OUPUT_PATH := ../user/target/riscv64gc-unknown-none-elf/$(MODE)


build:
	@for x in $(LIBS); do \
	 echo $$x; \
	 cd $$x; \
	 cargo build --$(MODE); \
	 cd ../; \
	done
	@$(foreach so, $(SO), cp $(so) $(patsubst $(TARGET_PATH)/lib%.so, $(OUPUT_PATH)/%, $(so));)

disasm:
	@$(foreach so, $(SO), $(OBJDUMP) -S $(so) > $(patsubst $(TARGET_PATH)/%.so, $(TARGET_PATH)/%.asm, $(so));)

clean:
	@rm -rf target

