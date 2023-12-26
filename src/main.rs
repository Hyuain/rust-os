#![no_std]   // do not link the Rust std library
#![no_main]  // disable all Rust-level entry points

use core::panic::PanicInfo;

static HELLO: &[u8] = b"Hello World!";

#[no_mangle] // do not mangle the name of this function
pub extern "C" fn _start() -> ! {
    // this is the entry point
    // the linker looks for a function named "_start" by default

    // the address of vga buffer is 0xb8000
    let vga_buffer = 0xb8000 as *mut u8;

    for (i, &byte) in HELLO.iter().enumerate() {
        // to access and dereference raw pointer, code should be wrapped by `unsafe`
        unsafe {
            // line width for qemu is 160
            let line_width: isize = 160;
            // last visible line of qemu is 24
            // first line is invisible, second line is behind the navbar
            // so start with the third line
            let line_offset: isize = line_width * 2;

            // each character cell consists of an ASCII byte and a color byte
            // set the offset of char and color in line
            let char_offset_within_line: isize = i as isize * 2;
            let color_offset_within_line: isize = i as isize * 2 + 1;

            let char_offset = char_offset_within_line + line_offset;
            let color_offset = color_offset_within_line + line_offset;

            // set value into addresses
            *vga_buffer.offset(char_offset) = byte;
            *vga_buffer.offset(color_offset) = 0xb;
        }
    }

    // a pause function to allow the text to be displayed
    unsafe {
        for _ in 0..5000000 {
        }
    }

    loop {}
}

/// This function is called on panic
#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    loop {}
}
