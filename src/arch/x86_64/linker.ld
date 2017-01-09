
/*
 * Linker Script
 */

ENTRY(start)

SECTIONS {
	/* Start the kernel at 1 Mb */
    . = 1M;

    .boot : {
        /* Ensure that the multiboot header is at the beginning */
        KEEP(*(.multiboot_header))
    }

    .text : {
        *(.text)
    }
}
