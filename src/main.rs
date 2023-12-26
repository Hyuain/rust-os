#![no_std]   // do not link the Rust std library
#![no_main]  // disable all Rust-level entry points

use core::panic::PanicInfo;

#[no_mangle] // do not mangle the name of this function
pub extern "C" fn _start() -> ! {
    // this is the entry point
    // the linker looks for a function named "_start" by default
    loop {}
}

/// This function is called on panic
#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    loop {}
}
