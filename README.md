# TODOS

Stack unwinding

# Introduction

This project aims to build a simple operating system using rust, following the [blog](https://os.phil-opp.com/) of Philipp Oppermann.

## S0: Freestanding Rust Binary

> The code of S0 and S1 can be found in the branch `s1-simple_kernel`.

### Create a Cargo Project

To create an OS kernel in Rust, we need to create an excutable that can be run without an underlying operating system. Such an excutable is often called **freestanding** or **bare-metal** excutable.

First create a new cargo project:

```bash
cargo new rust-os
```

By default, it will create a binary project with a `main.rs` file (instead of a library project). We can also add a `--bin` flag to create a binary project explicitly.

In the original blog, 2018 edition of Rust is used by specify `--edition 2018`. Here in this project however, 2021 edition is used.

We can use `cargo build` to build the create and check binary file in the `target/debug` folder. 

### Disable Std Library

The crate implicitly links the **standard library**, which should be disabled:

```rust
// main.rs

#![no_std]

fn main() {
    // `println!` marco is not available in no_std mode
    // println!("Hello, world!");
}
```

If we try to build the project now, we will get errors:

```bash
error: `#[panic_handler]` function required, but not found
error: language item required, but not found: `eh_personality`
```

So we need add `panic_handler` to deal with panic:

```rust
// in main.rs

use core::panic::PanicInfo;

/// This function is called on panic. And it is marked as a diverging function by returning `!` ("never" type)
#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    loop {}
}
```

`eh_personality` language item marks a function that is used for implementing stack unwinding.
It ensures that all memory is freed when panic and allows parent thread to catch the panic and recover from it. 
It is a complicated process, and we will not implement it here. We simply set the panic strategy to `abort` in `Cargo.toml` to avoid using unwinding and `eh_personality`:

```toml
[profile.dev]
panic = "abort"

[profile.release]
panic = "abort"
```

### Overwrite Entry Point 

Now we will get an error if we try to run `cargo build`:

```bash
error: requires `start` lang_item
```

This is because our program is missing the `start` language item, which defines the entry point.

Actually `main` is not the first function called when the program starts. Most language have a **runtime system** for garbage collection (e.g. in Java) or software threads (e.g. goroutines in Go). The runtime needs to be called before `main`.

In a typical Rust binary that links the std library, **execution starts in a C runtime** called `crt0`, which sets up the environment for a C application.

The C runtime then **invokes the [entry point of Rust runtime](https://github.com/rust-lang/rust/blob/bb4d1491466d8239a7a5fd68bd605e3276e97afb/src/libstd/rt.rs#L32-L73)**, which is marked by the `start` language item:

```rust
#[cfg(not(test))]
#[lang = "start"]
fn lang_start(main: fn(), argc: isize, argv: *const *const u8) -> isize {
    //...
    0
}
```

Rust has a very minimal runtime, taking care of setting up stack overflow guards and printing a backtrace on panic. The runtime then **finally calls the `main` function**.

The freestanding excutable does not have access to the Rust runtime and `ct0`, so we need to define our own entry point. We **can not simply implement `start` language item**, because the `crt0` is needed to call that function. We need to **overwrite `crt0` entry point directly**.

1. Add `![no_main]` attribute to tell the compiler not to use the normal entry point
2. Delete the `main` function, because there is no underlying runtime to call it
3. Add a new function named `_start` as a new entry point, this function should be marked as `#[no_mangle]` to prevent the compiler from changing the name of this function. `_start` is the default entry point name for most systems
4. Mark the function as `extern "C"` to tell the compiler that it should use the C calling convention for this function (instead of unspecified Rust calling convention)

```rust
#![no_std]

#[no_mangle]
pub extern "C" fn _start() -> ! {
    loop {}
}
```

### Linker Errors

No we all get ugly linker errors. The linker is a program that combines the generated code into an excutable. The default configuration of the linker assumes that our program depends on the C runtime, which is not true.

To solve the errors, we need to **build it for a bare metal target.** By default, Rust tries to build an executable that is able to run in the current system environment, for example, `.exe` when the build is run on Windows. The environment is called **host system**. We can check the host system by execute `rustc --version --verbose`.

To describe different environments, Rust uses a string called [target-triple](https://clang.llvm.org/docs/CrossCompilation.html#target-triple).

We can use set the target as `thumbv7em-none-eabihf` to build for a bare metal target. First add the target:

```bash
rustup target add thumbv7em-none-eabihf
```

Then use `--target` flag to set the target:

```bash
cargo build --target thumbv7em-none-eabihf
```

Since the target system has no operating system, the linker does not try to link the C runtime and our build succeeds.

## S1: A Minimal Rust Kernel

> The code of S0 and S1 can be found in the branch `s1-simple_kernel`.

### Booting Process

1. Turn on a computer
2. The firmware code that is stored in motherboard ROM is executed
3. The code performs a power-on self-test (POST), and looks for a bootable disk and stats booting the operating system kernel

On x86, there are two firmware standards: the **Basic Input/Output System (BIOS)** and the newer **Unified Extensible Firmware Interface (UEFI)**. Almost all x86 systems have support for BIOS booting, including newer UEFI-based machines that use an emulated BIOS.

But the wide compatibility is at the same time the biggest disadvantage of BIOS booting, because the CPU is put into a 16-bit compatibility mode called real mode before booting so that archiaic bootloaders from 1980s can still work.

For a BIOS Booting process:

1. Turn on a computer
2. The BIOS from some special flash memory on the motherboard is executed
3. The BIOS runs self-test and initialization routines of the hardware, and looks for bootable disks
4. If the BIOS finds one bootable disk, control is transferred to its **bootloader**, which is a 512-byte portion of executable code stored at the disk's beginning (Most bootloaders are larger than 512 bytes, bootloaders are commonly split into two stages, the first stage is 512 bytes and the second stage is subsequently loaded by the first stage)
5. The bootloader has to:
   1. determine the location of the kernel image on the disk and load it into memory
   2. switch the CPU from 16-bit real mode to the 32-bit protected mode, and then to the 64-bit long mode
   3. query certain information (such as a memory map) from the BIOS and pass it to the OS kernel

### Target Specification

To build a customized x86-64 operating system, we could define a target JSON file, for example `x86_64-rust_os.json`:

```json
{
  "llvm-target": "x86_64-unknown-none",
  "data-layout": "e-m:e-i64:64-f80:128-n8:16:32:64-S128",
  "arch": "x86_64",
  "target-endian": "little",
  "target-pointer-width": "64",
  "target-c-int-width": "32",
  "os": "none",
  "executables": true,
  "linker-flavor": "ld.lld",
  "linker": "rust-lld",
  "panic-strategy": "abort",
  "disable-redzone": true,
  "features": "-mmx,-sse,+soft-float"
}
```
And use `--target` to specify the target:

```bash
cargo build --target x86_64-blog_os.json
```

Now an error occurs:

```bash
error[E0463]: can't find crate for `core`
```

`core` library contains basic Rust types such as `Result`, `Option`, and iterators, and is implicitly linked to all `no_std` crates. The error is because the core library is distributed together with the Rust compiler as a precompiled library and only valid for supported host triples, not for our custom target. **So we need to recompile `core` first.**

The `build-std` feature of cargo allows to recompile `core` and other standard library crates on demand.

> NOTE: This feature can only be used in nightly Rust.

To use the feature, we need to create a local cargo configuration file at *.cargo/config.toml* (the *.cargo* folder should be next to the *src* folder):

```toml
# in .cargo/config.toml

[unstable]
# `compiler_builtins` crate is a dependency of `core`
build-std = ["core", "compiler_builtins"]
```

To recompile these libraries, cargo needs access to the rust source code:

```bash
rustup component add rust-src
```

We see that `cargo build` now recompiles the `core`, `rustc-std-workspace-core` (a dependency of `compiler_builtins`), and `compiler_builtins` libraries for our custom target.

Besides these libraries, we also need a set of built-in functions like `memset`, `memcpy` and `memcomp`, which are normally provided by the C library on the system. `compiler_builtins` crate already contains implementations for all the needed functions, they are disabled by default to not collide with the implementations from the C library. So we just need to use `build-std-features` to enable them:

```toml
# in .cargo/config.toml

[build]
# set the default target to avoid add --target every time
target = "x86_64-blog_os.json"

[unstable]
# this will add #[no_mangle] to those functions
build-std-features = ["compiler-builtins-mem"]
build-std = ["core", "compiler_builtins"]
```

### Printing to Screen

The easiest way to print text to the screen is the VGA text buffer. It is a special memory area mapped to the VGA hardware.

To print "Hello World!", we only need to know that the buffer is located at address 0x8000 and that each character cell consists of an ASCII byte and a color byte.

This is the code to print "Hello World!" to the screen:

```rust
fn print_hello_world() {
   // the address of vga buffer is 0xb8000
   let vga_buffer = 0xb8000 as *mut u8;

   for (i, &byte) in HELLO.iter().enumerate() {
      // to access and dereference raw pointer, code should be wrapped by `unsafe`
      unsafe {
         // line width for qemu is 80 cells (160 bytes)
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

         // set value into addresses, the unit for offset is byte
         *vga_buffer.offset(char_offset) = byte;
         *vga_buffer.offset(color_offset) = 0xb;
      }
   }
}
```

### Run the Kernel

First we need to create a bootable disk image.

1. Add `bootloader@0.9.23` dependency to the project. Make sure use the same version as the blog
2. Add `bootimage` tool to link the kernel with the bootloader and create a bootable disk image
3. Run the compiled kernel with QEMU

Firstly add dependency in *Cargo.toml*:

```toml
# in Cargo.toml

[dependencies]
bootloader = "0.9.23"
```

Then run the following command **outside the cargo project** to install `bootimage`:

```bash
cargo install bootimage
```

Then add `llvm-tools-preview` component to make `bootimage` work:

```bash
rustup component add llvm-tools-preview
```

Now we can return to the cargo project and use `bootimage` to create a bootable disk image:

```bash
cargo bootimage
```

The disk image is created at *target/x86_64-blog_os/debug/bootimage-rust_os.bin*.

To run the kernel, we can use the following command:

```bash
qemu-system-x86_64 -drive format=raw,file=target/x86_64-blog_os/debug/bootimage-rust_os.bin
```

To simplify the command, we can config cargo run in *.cargo/config.toml*:

```toml
# in .cargo/config.toml

[target.'cfg(target_os = "none")']
runner = "bootimage runner"
```

And then run the kernel with:

```bash
cargo run
```

## S2: VGA Text Mode

The code of this section can be found in branch `s2-vga_buffer`

The text buffer is a two-dimensional array with typically 25 rows and 80 columns. Content in the specified address is mapped to the VGA device (not mapped to RAM) and rendered to screen directly.

The array entry consists of two bytes:

The first byte represents characters that can be printed in the ASCII encoding (It actually is a character set named code page 437 with some slight modifications).

The second byte represents the color:

```text
from high to low:
1 bit for blink + 3 bits background color + 4 bits foreground color (include 1 bit for bright)
```

In the implementation, a 4 bit `Color` enum, a 8 bit `ColorCode` enum, a 8 bit `ascii_character` data and a 16 bit `ScreenChar` is used:

```rust
// color is to represent foreground or background color
#[repr(u8)]
pub enum Color {
    Black = 0,
    Blue = 1,
   // ...
}

// color code is the combination of foreground color and background color
#[repr(transparent)]
struct ColorCode(u8);

impl ColorCode {
   fn new(foreground: Color, background: Color) -> ColorCode {
      ColorCode((background as u8) << 4 | (foreground as u8))
   }
}

// use `repr(C)` to ensure that its fields are laid out in memory exactly like they would be in a C struct
// like `|ascii_character (1 byte) | color_code (1 byte)|`
// the attribute tell Rust do not optimize the layout, because the memory is directly mapped to the VGA Buffer,
// which need exactly correct data mapping
#[repr(C)]
struct ScreenChar {
   ascii_character: u8,
   color_code: ColorCode,
}

const BUFFER_HEIGHT: usize = 25;
const BUFFER_WIDTH: usize = 80;

// the struct only has one field
// and we want the struct is an exact and transparent representation of its underlying field
// its a two-dimensional array, and its mapped by row in memory
// the layout is exactly the same with the way we manipulated directly in the previous `print_hello_world` function
#[repr(transparent)]
struct Buffer {
   chars: [[ScreenChar; BUFFER_WIDTH]; BUFFER_HEIGHT],
}
```

And then a `Writer` struct is used for abstraction:

```rust
pub struct Writer {
    column_position: usize,
    color_code: ColorCode,
    buffer: &'static mut Buffer,
}

impl Writer {
   pub fn write_byte(&mut self, byte:u8) {
      // ...
      self.buffer.chars[row][col] = ScreenChar {
         ascii_character: byte,
         color_code: self.color_code,
      };
      self.column_position += 1;
   }
   
   pub fn write_string(&mut self, s: &str) {
      for byte in s.bytes() {
         match byte {
            0x20..=0x7e | b'\n' => self.write_byte(byte),
            _ => self.write_byte(0xfe),
         }
      }
   }
   
   fn new_line(&mut self) {
      
   }
}
```

To use the struct, we can simply create a instance, and call its methods:

```rust
pub fn print_something() {
   let mut writer = Writer {
      column_position: 0,
      color_code: ColorCode::new(Color::Yello, Color::Black),
      // here we cast the integer 0x8000 as a raw pointer using (as *mut Buffer)
      // then use * to dereference, then got a Buffer
      // and then use &mut to borrow it, got a mutable pointer
      // so that in the later section we can use it as a typical Rust reference (&mut Buffer)
      buffer: unsafe { &mut *(0xb8000 as *mut Buffer) },
   };
   writer.write_byte(b'H');
   writer.write_string("ello ");
   writer.write_string("Wörld!");
}
```

Here we just write to the `Buffer` and never read from it again, the compiler does not know we really access VGA buffer memory, so it might decide that these writes are unnecessary and can be omiited.

To avoid those erroneous optimization, we need to specify these writes as [volatile](https://en.wikipedia.org/wiki/Volatile_(computer_programming)). A `volatile` crate is used to wrap the struct.

NOTE to specify `volatile` version as `0.2.6` to make sure it compatible with this project.

```rust
use volatile::Volatile;

struct Buffer {
    chars: [[Volatile<ScreenChar>; BUFFER_WIDTH]; BUFFER_HEIGHT],
}

impl Writer {
   pub fn write_byte(&mut self, byte: u8) {
      // ..
      self.buffer.chars[row][col].write(ScreenChar {
         ascii_character: byte,
         color_code,
      });
   }
}
```

