
//
//  Kernel Main
//

#![feature(lang_items)]
#![no_std]

#[no_mangle]
pub extern fn kernel_main() {

}

#[lang = "eh_personality"]
extern fn eh_personality() {
	// Do nothing for now
}

#[lang = "panic_fmt"]
#[no_mangle]
pub extern fn panic_fmt() -> ! {
	// Make sure this function doesn't return (required by the ! return type)
	loop {}
}
