
/*
 * Linker Script
 */

/* Set the main entry point for our kernel to be a symbol named `start` */
ENTRY(start)

SECTIONS {
	/* Start the kernel at 1 MB since a lot of hardware devices and bootloader
	 * information is mapped to memory below 1 MB (eg. the VGA memory at
	 * 0xb8000) */
    . = 1M;

    .boot : {
        /* Ensure that the multiboot header is at the beginning */
        KEEP(*(.multiboot_header))
    }

	.text : {
		/* Merge all .text.* subsections into the .text section */
		*(.text .text.*)
	}

	.rodata : {
		/* Merge all .rodata.* subsections into the .rodata section */
		*(.rodata .rodata.*)
	}

	.data.rel.ro : {
		/* Merge all .data.rel.ro.* subsections into the .data.rel.ro section */
		*(.data.rel.ro.local*) *(.data.rel.ro .data.rel.ro.*)
	}
}
