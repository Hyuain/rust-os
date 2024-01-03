#![no_std]   // do not link the Rust std library
#![no_main]  // disable all Rust-level entry points

use core::fmt::Write;
use core::panic::PanicInfo;

mod vga_buffer;

#[no_mangle] // do not mangle the name of this function
pub extern "C" fn _start() -> ! {
    println!("Hello World{}", "!");
    panic!("This is a panic message");
    loop {}
}

/// This function is called on panic
#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    println!("{}", info);
    loop {}
}
