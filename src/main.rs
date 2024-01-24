#![no_std] // do not link the Rust std library
#![no_main] // disable all Rust-level entry points
#![feature(custom_test_frameworks)] // replace the default test framework which relies on std lib
#![test_runner(rust_os::test_runner)]
// the custom test framework feature generates a main function that calls test_runner, which is ignored by `![no_main]`
#![reexport_test_harness_main = "test_main"]

extern crate alloc;

use alloc::boxed::Box;
use bootloader::{entry_point, BootInfo};
use core::panic::PanicInfo;
use x86_64::structures::paging::{Page, PageTable, Translate};
use x86_64::VirtAddr;

use rust_os::memory::BootInfoFrameAllocator;
use rust_os::{memory, print, println};

entry_point!(kernel_main);

fn kernel_main(boot_info: &'static BootInfo) -> ! {
    println!("Hello World{}", "!");

    rust_os::init();

    /* Test memory mapping */

    let phys_mem_offset = VirtAddr::new(boot_info.physical_memory_offset);
    let mut mapper = unsafe { memory::init(phys_mem_offset) };
    let mut frame_allocator = unsafe { BootInfoFrameAllocator::init(&boot_info.memory_map) };
    // let mut frame_allocator = memory::EmptyFrameAllocator;

    let page = Page::containing_address(VirtAddr::new(0xdeadbeaf000));
    memory::create_example_mapping(page, &mut mapper, &mut frame_allocator);

    let page_ptr: *mut u64 = page.start_address().as_mut_ptr();
    unsafe { page_ptr.offset(400).write_volatile(0x_f021_f077_f065_f04e) };

    /* Test heap allocation */

    let x = Box::new(41);

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
