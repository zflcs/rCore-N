
TARGET := riscv64gc-unknown-linux-gnu
MODE := release
OBJDUMP := riscv64-unknown-linux-gnu-objdump

BUILD_PATH := ../../target/$(TARGET)/$(MODE)
ELF := $(BUILD_PATH)/libsharedscheduler.so
ASM := $(BUILD_PATH)/libsharedscheduler.asm


clean:
	cargo clean

build:
	cargo build --release
	$(OBJDUMP) -S -t $(ELF) > $(ASM)


disasm: build
	$(OBJDUMP) -S -t $(ELF) > $(ASM)