
#
#  Kernel Makefile
#

arch ?= x86_64
target ?= $(arch)-unknown-linux-gnu
rust_lib := target/$(target)/debug/libcanary.a
kernel := build/canary-$(arch).bin
iso := build/canary-$(arch).iso

linker_script := src/arch/$(arch)/linker.ld
grub_cfg := src/arch/$(arch)/grub.cfg

assembly_source_files := $(wildcard src/arch/$(arch)/*.asm)
assembly_object_files := $(patsubst src/arch/$(arch)/%.asm, build/arch/$(arch)/%.o, $(assembly_source_files))

.PHONY: all clean run iso

all: $(kernel) $(iso)

clean:
	@rm -r build

run:
	qemu-system-x86_64 -cdrom $(iso)

debug:
	qemu-system-x86_64 -d int -no-reboot -cdrom $(iso)

iso: $(iso)

cargo:
	cargo build --target $(target)

$(iso): $(kernel) $(grub_cfg)
	@mkdir -p build/iso/boot/grub
	@cp $(kernel) build/iso/boot/kernel.bin
	@cp $(grub_cfg) build/iso/boot/grub
	grub-mkrescue -o $(iso) build/iso
	@rm -r build/iso

$(kernel): cargo $(rust_lib) $(assembly_object_files) $(linker_script)
	ld -n --gc-sections -T $(linker_script) -o $(kernel) $(assembly_object_files) $(rust_lib)

build/arch/$(arch)/%.o: src/arch/$(arch)/%.asm
	@mkdir -p $(shell dirname $@)
	nasm -felf64 $< -o $@
