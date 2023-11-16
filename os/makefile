# Building
QEMU := ../../qemu-build/riscv64-softmmu/qemu-system-riscv64

TARGET := riscv64gc-unknown-none-elf
MODE := release
OBJDUMP := riscv64-unknown-linux-gnu-objdump
OBJCOPY := riscv64-unknown-linux-gnu-objcopy

BUILD_PATH := ../target/$(TARGET)/$(MODE)
KERNEL_ELF := $(BUILD_PATH)/os
KERNEL_ASM := $(BUILD_PATH)/os.asm
KERNEL_BIN := $(BUILD_PATH)/os.bin
KERNEL_BIN_axu15eg := $(BUILD_PATH)/rcore-n.bin

APPS := ../user_lib/src/bin/*
FS_IMG := ../target/$(TARGET)/$(MODE)/fs.img


clean:
	@cargo clean


user: $(APPS)
	@cd ../user_lib && make build
	@rm -f $(FS_IMG)
	@cd ../easy-fs-fuse && cargo run --release -- -s ../user_lib/src/bin/ -t ../target/riscv64gc-unknown-linux-gnu/release/

build: user
	@LOG=DEBUG cargo build --features board_qemu --release
	@$(OBJCOPY) $(KERNEL_ELF) --strip-all -O binary $(KERNEL_BIN)

build_axu15eg: user_axu15eg
	@LOG=DEBUG cargo build --features board_axu15eg --release
	@$(OBJCOPY) $(KERNEL_ELF) --strip-all -O binary $(KERNEL_BIN)
	@cp $(KERNEL_BIN) $(KERNEL_BIN_axu15eg)


disasm: build
	@$(OBJDUMP) -S -t $(KERNEL_ELF) > $(KERNEL_ASM)

disasm_axu15eg: build_axu15eg
	@$(OBJDUMP) -S -t $(KERNEL_ELF) > $(KERNEL_ASM)

run: build
	@cd ../opensbi && make CROSS_COMPILE=riscv64-unknown-elf- PLATFORM=generic
	@$(QEMU) -machine virt -smp 4  -nographic -bios ../opensbi/build/platform/generic/firmware/fw_payload.elf \
	-device virtio-net-device,netdev=net0 \
	-netdev user,id=net0,hostfwd=tcp::6201-:80 -d in_asm -D log.txt

upload: build_axu15eg
	@cd /home/zfl/u-intr/opensbi && make CROSS_COMPILE=riscv64-unknown-elf- PLATFORM=axu15eg && \
	scp build/platform/axu15eg/firmware/fw_payload.bin axu15eg:~
	@ssh axu15eg ./start_rocket.sh

# debug_qemu: build
#     $(QEMU) -machine virt -smp 4 $(SERIAL_FLAGS) -nographic -bios ./rustsbi-qemu.bin -device loader,file=$(KERNEL_BIN),addr=0x80200000 -d int -D debug.log

# debug: build disasm
#     tmux new-session -d $(QEMU) -machine virt -smp 4 $(SERIAL_FLAGS) -nographic -bios ./rustsbi-qemu.bin -device loader,file=$(KERNEL_BIN),addr=0x80200000 -s -S && tmux split-window -h riscv64-unknown-elf-gdb -ex 'file $(KERNEL_ELF)' -ex 'set arch riscv:rv64' -ex 'target remote localhost:1234' && tmux -2 attach-session -d

# debug_all: build disasm
#     tmux new-session -d $(QEMU) -machine virt -smp 4 $(SERIAL_FLAGS) -nographic -bios ./rustsbi-qemu.bin -device loader,file=$(KERNEL_BIN),addr=0x80200000 -d int -D debug.log -s -S && tmux split-window -h riscv64-unknown-elf-gdb -ex 'file $(KERNEL_ELF)' -ex 'set arch riscv:rv64' -ex 'target remote localhost:1234' && tmux -2 attach-session -d

.PHONY: run upload disasm disasm_axu15eg build clean