#![no_std]
#![cfg_attr(test, no_main)]
#![feature(custom_test_frameworks)]
#![feature(abi_x86_interrupt)]
#![test_runner(crate::test_runner)]
#![reexport_test_harness_main = "test_main"]

use core::panic::PanicInfo;
use x86_64::instructions::port::Port;

pub mod serial;
pub mod vga_buffer;
pub mod interrupts;

pub trait Testable {
    fn run(&self) -> ();
}

impl<T> Testable for T where T: Fn() {
    fn run(&self) -> () {
        serial_print!("{}...\t", core::any::type_name::<T>());
        self();
        serial_println!("[ok]");
    }
}

// include this function only for tests
// &[&dyn Testable] a slice of trait object references of the Testable trait  -> the slice will contains references to function marked as test_case
// because the trick implementation of Testable, any type that can be called like a function (i.e., implements the Fn() trait) also automatically implements the Testable trait
// It is a list of references to types that can be called like a function
pub fn test_runner(tests: &[&dyn Testable]) {
    serial_println!("Running {} tests", tests.len());
    for test in tests {
        test.run();
    }
    exit_qemu(QemuExitCode::Success);
}

pub fn test_panic_handler(info: &PanicInfo) -> ! {
    serial_println!("[failed]\n");
    serial_println!("Error: {}\n", info);
    exit_qemu(QemuExitCode::Failed);
    loop {}
}

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
// because iosize of the isa-debug-exit devices is specified as 4 bytes
#[repr(u32)]
pub enum QemuExitCode {
    // the values is chosen casually as long as they do not clash with the default exit codes of QEMU
    // for example, if we choose 0 as success, it will become (0 << 1) | 1 = 1 after transformation
    // which is the default exit code when QEMU fails to run
    Success = 0x10,
    Failed = 0x11,
}

pub fn exit_qemu(exit_code: QemuExitCode) {
    unsafe {
        // port 0xf4 is the iobase of the isa-debug-exit device
        // if `value` is written to its iobase, qemu exits with exit status `(value << 1) | 1`
        let mut port = Port::new(0xf4);
        port.write(exit_code as u32);
    }
}

/// Entry point for `cargo test`
#[cfg(test)]
#[no_mangle]
pub extern "C" fn _start() -> ! {
    test_main();
    loop {}
}

#[cfg(test)]
#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    test_panic_handler(info)
}

pub fn init() {
    interrupts::init_idt();
}
