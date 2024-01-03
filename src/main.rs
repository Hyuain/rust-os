#![no_std]   // do not link the Rust std library
#![no_main]  // disable all Rust-level entry points

use core::fmt::Write;
use core::panic::PanicInfo;
use crate::vga_buffer::WRITER;

mod vga_buffer;

#[no_mangle] // do not mangle the name of this function
pub extern "C" fn _start() -> ! {
    WRITER.lock().write_str("Hello again").unwrap();
    write!(WRITER.lock(), ", some numbers: {} {}", 42, 1.337).unwrap();

    loop {}
}

/// This function is called on panic
#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    loop {}
}
