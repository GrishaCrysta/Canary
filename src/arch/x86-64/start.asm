
;
;  Kernel Entry Point
;

bits 32

section .bss

; Switching to long mode in x86 requires us to enable paging, so for the kernel
; to still work we need to set up some page tables before we switch modes.
;
; Allocate memory for a preliminary set of page tables, used to identity map
; the kernel so that the code still works after we switch to long mode.
;
; We're going to identity map a 1 GB region of memory starting at 0x0 and
; finishing at 0x40000000 (1 GB in hex). Normally, x86 uses 4 page tables and a
; page size of 4096 bytes, but we can map 2 MB pages using only 3 page tables
; (P4, P3, and P2), and enabling certain bits in the page table entry flags in
; P2. So we're going to map 1, 2 MB page at 0x0, the next starting at 0x200000
; (2 MB), the next at 0x400000 (4 MB), and so on...
;
; Page tables need to be aligned on page boundaries, so align this entire
; section to the size of a page (4096 bytes)
align 4096
p4_table:
	resb 4096
p3_table:
	resb 4096
p2_table:
	resb 4096

; Allocate some memory used for the kernel's stack, which we need for register
; overflow and several CPU feature checks (since `eflags` can only be pushed to
; the stack)
stack_bottom:
	resb 64
stack_top:


section .text

; Prints an error message and error number to the screen and hangs.
; The error code (an ASCII value) should be in `al`
error:
	mov dword [0xb8000], 0x4f524f45
	mov dword [0xb8004], 0x4f3a4f52
	mov dword [0xb8008], 0x4f204f20
	mov byte  [0xb800a], al
	hlt


; Check that the kernel was loaded by a multiboot-compliant bootloader, because
; we rely on some Multiboot-specific functions later on (such as determining
; the location of the kernel in memory, etc).
check_multiboot:
	; If the kernel was loaded by a multiboot-compliant bootloader, then the
	; magic value `0x36d76289` will be in `eax`
	cmp eax, 0x36d76289
	jne .no_multiboot
	ret

	; Error handling if the kernel wasn't loaded by a multiboot-compliant
	; bootloader
.no_multiboot:
	mov al, "0"
	jmp error


; We need to use the `cpuid` instruction later to get various pieces of
; information about the CPU. This instruction isn't supported on all CPUs, so
; we need to check if we can use it or not.
;
; `cpuid` is supported if we can flip the ID bit (bit 21) in the `flags`
; register.
check_cpuid:
	; We can't directly access or modify the flags register, but we can push it
	; to the stack using `pushfd`. So for us to flip bit 21 in the `eflags`
	; register, we need to:
	;
	; 1. Push it to the stack (`pushfd`)
	; 2. Load it into a general purpose register (`eax`)
	; 3. Flip the ID bit in the general purpose register
	; 4. Push the general purpose register onto the stack
	; 5. Update the `eflags` register by popping a value of the stack (`popfd`)

	; Push `eflags` onto the stack
	pushfd

	; Pop the `eflags` register off the stack into `eax`
	pop eax

	; Make a copy of `eflags` for later comparison
	mov ecx, eax

	; Flip the ID bit (bit 21)
	xor eax, 1 << 21

	; Copy `eax` into `eflags` via the stack
	push eax
	popfd

	; Copy `eflags` back into `eax`. If we're not allowed to flip bit 21, then
	; this value will be different than the one we just stored into `eflags`
	pushfd
	pop eax

	; If `eax` and `ecx` are different, then the CPU didn't let us flip the ID
	; bit, and so the `cpuid` instruction isn't supported
	cmp eax, ecx
	je .no_cpuid

	; If we were successfully able to flip the ID bit, we don't actually want
	; to keep it flipped when we return from this function, so we need to
	; restore the original value of the `eflags` register using the value we
	; copied into `ecx`
	push ecx
	popfd

	ret

	; Error handling if `cpuid` isn't supported
.no_cpuid:
	mov al, "1"
	jmp error


; Long mode is a setting for the CPU which allows it to execute 64 bit
; instructions (but doesn't quite yet, there's a bit more setup that's
; required). We need to check if it's supported before we can enable it,
; which we do using the `cpuid` instruction.
;
; Assumes that the `check_cpuid` check has been run before calling this
; function.
check_long_mode:
	; To check if long mode is enabled, we call `cpuid` with `0x80000001` in
	; `eax`. But some CPUs don't support this check. So first, we need to check
	; if the CPU even supports checking if long mode exists

	; To check if the long mode check exists, we get the highest supported
	; value of `eax` that this CPU supports. We do this by calling `cpuid` with
	; `0x80000000` in `eax`
	mov eax, 0x80000000
	cpuid

	; The highest supported value of `eax` needs to be at least `0x80000001`
	cmp eax, 0x80000001
	jb .no_long_mode

	; Now actually check if long mode is supported
	mov eax, 0x80000001
	cpuid
	test edx, 1 << 29
	jz .no_long_mode
	ret

	; Error handling if long mode isn't supported
.no_long_mode:
	mov al, "2"
	jmp error


; Sets up a series of page tables so that the first 1 GB of virtual memory is
; identity mapped to the first 1 GB of physical memory.
setup_page_tables:
	; By default, GRUB fills all memory in the .bss section with 0s when it
	; loads the kernel (despite the .bss section typically being uninitialised).
	; This means all 3 page tables are already valid (containing all 0s), but
	; aren't very useful to us yet because they don't actually do anything

	; Map the first entry in the P4 table to the P3 table
	; Set the lower 2 bits to indicate the P3 page table is present in memory
	; and is writable
	mov eax, p3_table
	or eax, 11b
	mov [p4_table], eax

	; Map the first entry in the P3 table to the P2 table
	; Again, set the present and writable flags
	mov eax, p2_table
	or eax, 11b
	mov [p3_table], eax

	; Use a loop to map the `ecx`th entry in the P2 table to a region of memory
	; starting at `ecx` * 0x200000 (2 MB) and of size 2 MB (using special
	; bit flags in the page table entry)
	mov ecx, 0 ; Use `ecx` as the loop counter
.set_p2_entry:
	mov eax, 0x200000 ; 2 MB
	mul ecx           ; `eax` = `eax` * given register (`ecx`)
	or eax, 10000011b ; Set the present, writable, and "huge" (2 MB page) flags
	mov [p2_table + ecx * 8], eax

	; Stop the loop when we've filled all 512 page table entries
	inc ecx
	cmp ecx, 512
	jne .set_p2_entry

	ret


; Switches to long mode and enables paging. More specifically, this function
; tells the CPU where the P4 page table is, sets some bits to enable long mode,
; then sets another bit to enable paging.
switch_to_long_mode:
	; The CPU looks in the cr3 register to find the address of the P4 page
	; table, so load our P4 table into this register
	;
	; Normally, accessing/modifying the cr3 register in user mode is a
	; restricted operation, but we're a kernel running in kernel mode so it's
	; fine
	mov eax, p4_table
	mov cr3, eax

	; Long mode is part of the "Physical Address Extension" (PAE) x86 CPU
	; extension, which we need to enable by setting bit 5 in the cr4 register
	;
	; We can't directly modify the cr4 register, so we need to move it to a
	; general purpose register (`eax`) first
	mov eax, cr4
	or eax, 1 << 5
	mov cr4, eax

	; To enable long mode, we set bit 8 in the EFER MSR register
	; Again, we have to use a temporary general purpose register
	mov ecx, 0xC0000080
	rdmsr
	or eax, 1 << 8
	wrmsr

	; Finally, enable paging by setting the 31st bit in the cr0 register
	; Again, use a temporary general purpose register
	mov eax, cr0
	or eax, 1 << 31
	mov cr0, eax

	ret


; The kernel's main entry point, jumped to by the bootloader.
global start
start:
	; Update the stack pointer to point to our empty kernel stack
	; Use the `stack_top` since the stack has to grow downwards
	mov esp, stack_top

	; Ensure various CPU features are supported
	call check_multiboot
	call check_cpuid
	call check_long_mode

	; Setup paging and switch to long mode
	call setup_page_tables
	call switch_to_long_mode

	; Print `OK` to screen
	;
	; We can use these instructions (written in 32 bit mode) even though we're
	; in 64 bit mode because their corresponding machine code is the same on
	; both architectures
	mov dword [0xb8000], 0x2f4b2f4f
	hlt
