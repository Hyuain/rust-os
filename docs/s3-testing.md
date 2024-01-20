# S3: Testing

## Create a Library

In this project, both unit testing and integration testing are used. The integration tests run in separate environment, and it can not access functions defined in the `main.rs` file. So to avoid code reduplication, we created a library `lib.rs`, and move useful functions there, making them can be available both in `main.rs` and integration tests under _tests_ directory.

The `lib.rs` is tested independently of our `main.rs`, so we need to add a `_start` entry point and a panic handler when the library is compiled in test mode.

```rust
// in src/lib.rs
#![cfg_attr(test, no_main)]

use core::panic::PanicInfo;

/// Entry point for `cargo test`
#[cfg(test)]
#[no_mangle]
pub extern "C" fn _start() -> ! {
    loop {}
}

#[cfg(test)]
#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    loop {}
}
```

The first argument of `cfg_attr` is a condition, the second is an attribute. This line means that if the code is being compiled in a test context (i.e., `test` is true), then the `no_main` attribute should be applied.

This ensures that when the library is compiled for tests, it uses a custom entry point (`_start`) instead of the standard `main` function.

NOTE that the **libraries** typically do not have a main function, because they are not standalone executables. In normal build mode, the library is just a collection of functions, types, traits, etc.

However, during testing, the test harness may require a different entry point than the usual function. In that situation, the `#![cfg(test, no_main)]` attribute is used to tell the compiler to use a custom `_start`. 

`#[cfg(test)]` make sure these functions be included only in test mode.

## Custom Test Frameworks

The details can be found in the [original blog](https://os.phil-opp.com/testing/#custom-test-frameworks)

Here are some key points:

Use `#![feature(custom_test_frameworks)]` to specify that the default test frameworks shall be used instead of the default `test` crate which depends on the standard library.

And the `#![test_runner(crate::test_runner)]` attribute to specify the `test_runner` function. This way, all functions annotated with a `#[test_case]` attribute are collected and passed to `test_runner` during testing.

The `custom_test_frameworks` feature then generates a `main` function that calls `test_runner`, but it is ignored under `#![cfg(test, no_main)]` or `#![no_main]`. So the `#![reexport_test_harness_main = "test_main"]` attributed is used to set the name of the generated entry point of the test framework to `test_main` and call it manually in our `_start` function.

## I/O Ports

There are two different approaches for communicating between the CPU and peripheral hardware on x86, **memory-mapped I/O** and **port-mapped I/O**.

- Memory-mapped I/O is used for accessing the VGA text buffer through the memory address `0xb8000`
- Port-mapped I/O uses a separate I/O bus, and each peripheral has one or more port numbers. To communicate with such an I/O port, there are special CPU instructions called `IN` and `OUT`

### Exit QEMU

To exit QEMU, a special `isa-debug-exit` device is utilized. It is supported by QEMU, and will exit QEMU when we write some value to its I/O port.

To add the device in test environment, we add the following command to _Cargo.toml_:

```toml
# in Cargo.toml

[package.metadata.bootimage]
test-arg = ["-device", "isa-debug-exit,iobase=0xf4,iosize=0x04"]
```

And utilize `x86_64` crate to write value to the port `0xf4` which we specified above:

```rust
pub fn exit_qemu(exit_code: QemuExitCode) {
    use x86_64::instructions::port::Port;
    
    unsafe {
        let mut port = Port::new(0xf4);
        port.write(exit_code as u32);
    }
}
```

### Print to Console

Similarly, we use port-mapped I/O to print things to console. Here a easy-programmable [serial port](https://en.wikipedia.org/wiki/Serial_port) is used. The chips implementing a serial interface are called UARTs. The common UARTs today are all compatible with the 16550 UART. We will use the `uart_16550` crate to initialize the UART and send data over the serial port.

```rust
// in src/serial.rs

use uart_16550::SerialPort;
use spin::Mutex;
use lazy_static::lazy_static;

lazy_static! {
    pub static ref SERIAL1: Mutex<SerialPort> = {
        // 0x3F8 is the standard port number for the first serial interface
        let mut serial_port = unsafe { SerialPort::new(0x3F8) };
        serial_port.init();
        Mutex::new(serial_port)
    };
}

#[doc(hidden)]
pub fn _print(args: ::core::fmt::Arguments) {
    use core::fmt::Write;
    SERIAL1.lock().write_fmt(args).expect("Printing to serial failed");
}

/// Prints to the host through the serial interface.
#[macro_export]
macro_rules! serial_print {
    ($($arg:tt)*) => {
        $crate::serial::_print(format_args!($($arg)*));
    };
}

/// Prints to the host through the serial interface, appending a newline.
#[macro_export]
macro_rules! serial_println {
    () => ($crate::serial_print!("\n"));
    ($fmt:expr) => ($crate::serial_print!(concat!($fmt, "\n")));
    ($fmt:expr, $($arg:tt)*) => ($crate::serial_print!(
        concat!($fmt, "\n"), $($arg)*));
}
```

And we use `-serial` argument to redirect the output to stdout:

```toml
# in Cargo.toml

[package.metadata.bootimage]
test-args = [
    "-device", "isa-debug-exit,iobase=0xf4,iosize=0x04", "-serial", "stdio"
]
```
