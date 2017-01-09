
;
;  Kernel Entry Point
;

bits 32

section .bss

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
; instructions. We need to check if it's supported before we can enable it,
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

	; Print `OK` to screen
	mov dword [0xb8000], 0x2f4b2f4f
	hlt
