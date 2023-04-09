OBJDUMP := rust-objdump --arch-name=riscv64
TARGET := riscv64gc-unknown-linux-gnu
ELF := target/$(TARGET)/release/sharedscheduler
DISASM_TMP := target/$(TARGET)/release/sharedscheduler.asm


build:
	cargo build --release --features inner
	@$(OBJDUMP) -S $(ELF) > $(DISASM_TMP)
	cp target/$(TARGET)/release/sharedscheduler \
        ../user/target/riscv64gc-unknown-none-elf/release/sharedscheduler

build_test:
	cargo build --release --features inner
	@$(OBJDUMP) -S $(ELF) > $(DISASM_TMP)
	cp target/$(TARGET)/release/sharedscheduler \
        ../utest/target/riscv64gc-unknown-none-elf/release/sharedscheduler
