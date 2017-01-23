
/*
 * Linker Script
 */

ENTRY(start)

SECTIONS {
	/* Within the linker script, we have to make a distinction between physical
	 * and virtual memory. We need all relocation addresses within the generated
	 * ELF executables to use virtual addresses (ie. starting at
	 * 0xffff800000100000). But we need to physically place the kernel starting
	 * at only 0x100000.
	 *
	 * We achieve this by letting `.` represent virtual addresses and using
	 * the `AT` command to specify physical addresses.
	 */

	/* Start the kernel not at 0xffff800000000000 (ie. physical memory 0x0) but
	 * at 0xffff800000100000 (ie. physical memory 0x100000, or 1 MB).
	 *
	 * We do this because there's a bunch of IO devices mapped starting at
	 * 640 KB. If we mapped the kernel at 0x0, we'd have to split it in half if
	 * it were larger than 640 KB.
	 */
	. = 0xffff800000100000;

	/* The `PROVIDE` function defines an exported, external symbol that can be
	 * referenced within our Rust or assembly code.
	 *
	 * Throughout the linker script, we let Rust know where in virtual memory
	 * each of the ELF sections are. We then use this information to configure
	 * the correct flags in the page table entries for the kernel so that (for
	 * example) we can't write to the `.rodata` section.
	 */
	PROVIDE(kernel_start = .);

	/* Start placing sections in physical memory at 0x100000, ie. 1 MB */
	.boot : AT(0x100000) {
		/* Ensure that the multiboot header is at the very beginning, so that
		 * the bootloader recognises us as a valid OS.
		 *
		 * We supply the `--gc-sections` argument on the command line, which
		 * would get rid of the multiboot section normally, since it's not
		 * referenced anywhere else in the program. We use the `KEEP` function
		 * to prevent the linker from removing it.
		 */
		KEEP(*(.multiboot))
	}

	/* Because we want to set specific flags on all pages that contain
	 * executable instructions, this data itself must be page-aligned.
	 *
	 * 0x1000 (ie. 4096 bytes) is the size of a page.
	 */
	. = ALIGN(0x1000);

	PROVIDE(kernel_text_start = .);
	.text : {
		/* Merge all `.text.*` subsections into the .text section */
		*(.text .text.*)
	}

	/* Again, page align the read-only data sections */
	. = ALIGN(0x1000);

	PROVIDE(kernel_rodata_start = .);
	.rodata : {
		/* Merge all `.rodata.*` subsections into the .rodata section */
		*(.rodata .rodata.*)
	}

	.data.rel.ro : {
		/* Merge all `.data.rel.ro.*` subsections into `.data.rel.ro` */
		*(.data.rel.ro.local*) *(.data.rel.ro .data.rel.ro.*)
	}

	/* Again, page align the data sections */
	. = ALIGN(0x1000);

	PROVIDE(kernel_data_start = .);
	.data : {
		*(.data)
	}

	.bss : {
		*(.bss)
	}

	PROVIDE(kernel_end = .);
}
