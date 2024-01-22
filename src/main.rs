#![no_std] // do not link the Rust std library
#![no_main] // disable all Rust-level entry points
#![feature(custom_test_frameworks)] // replace the default test framework which relies on std lib
#![test_runner(rust_os::test_runner)]
// the custom test framework feature generates a main function that calls test_runner, which is ignored by `![no_main]`
#![reexport_test_harness_main = "test_main"]

use bootloader::{entry_point, BootInfo};
use core::panic::PanicInfo;
use x86_64::registers::control::Cr3;
use x86_64::structures::paging::{PageTable, Translate};
use x86_64::VirtAddr;

use rust_os::{memory, print, println};

entry_point!(kernel_main);

fn kernel_main(boot_info: &'static BootInfo) -> ! {
    println!("Hello World{}", "!");

    rust_os::init();

    let phys_mem_offset = VirtAddr::new(boot_info.physical_memory_offset);
    let mapper = unsafe { memory::init(phys_mem_offset) };

    let addresses = [
        // the identity-mapped vga buffer page
        0xb8000,
        // some code page
        0x201008,
        // some stack page
        0x0100_0020_1a10,
        // virtual address mapped to physical address 0
        boot_info.physical_memory_offset,
    ];

    for &address in &addresses {
        let virt = VirtAddr::new(address);
        let phys = mapper.translate_addr(virt);
        println!("{:?} -> {:?}", virt, phys);
    }

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
