
//
//  Kernel Main
//

#![feature(lang_items)]
#![no_std]

// The Rust `core` crate has a number of basic dependencies like `memcpy`,
// `memset`, etc. In order to provide these symbols, we include a Rust
// implementation of them in an external library (instead of having to write
// them ourselves).
extern crate rlibc;

// This is the main Rust entry point for the kernel, called from the `start.asm`
// code after a bunch of configuration (like switching to long mode) is done.
#[no_mangle]
pub extern fn kernel_main() {

}

#[lang = "eh_personality"]
extern fn eh_personality() {
	// Do nothing for now
}

// This is called when a Rust function calls the `panic!` macro, and should
// print an error message and not return.
#[lang = "panic_fmt"]
#[no_mangle]
pub extern fn panic_fmt() -> ! {
	// Make sure this function doesn't return (required by the ! return type)
	loop {}
}
