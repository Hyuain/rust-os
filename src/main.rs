#![no_std] // do not link the Rust std library
#![no_main] // disable all Rust-level entry points
#![feature(custom_test_frameworks)] // replace the default test framework which relies on std lib
#![test_runner(rust_os::test_runner)]
// the custom test framework feature generates a main function that calls test_runner, which is ignored by `![no_main]`
#![reexport_test_harness_main = "test_main"]

use core::panic::PanicInfo;
use x86_64::registers::control::Cr3;

use rust_os::{print, println};

#[no_mangle] // do not mangle the name of this function
pub extern "C" fn _start() -> ! {
    println!("Hello World{}", "!");

    rust_os::init();

    let (level_4_page_table, _) = Cr3::read();
    println!(
        "Level 4 page table at: {:?}",
        level_4_page_table.start_address()
    );

    // invoke a breakpoint exception
    // x86_64::instructions::interrupts::int3();

    // this will only be included for tests
    // and `test_main` function will be generated by the custom_test_frameworks feature
    #[cfg(test)]
    test_main();

    println!("I did not crash!");
    rust_os::hlt_loop();
}

/// This function is called on panic
#[cfg(not(test))] // when not in test mode
#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    println!("{}", info);
    rust_os::hlt_loop();
}

#[cfg(test)]
#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    rust_os::test_panic_handler(info)
}
