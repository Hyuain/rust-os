# S2: VGA Text Mode

## A Writer Struct

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

    // move every character one line up (the top line gets deleted),
    // and start at the beginning of the last line again
    fn new_line(&mut self) {
        for row in 1..BUFFER_HEIGHT {
            for col in 0..BUFFER_WIDTH {
                let character = self.buffer.chars[row][col];
                self.buffer.chars[row - 1][col] = character;
            }
        }
        self.clear_row(BUFFER_HEIGHT - 1);
        self.column_position = 0;
    }

    fn clear_row(&mut self, row: usize) {
        let blank = ScreenChar {
            ascii_character: b' ',
            color_code: self.color_code,
        };
        for col in 0..BUFFER_WIDTH {
            self.buffer.chars[row][col] = blank;
        }
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

Here we just write to the `Buffer` and never read from it again, the compiler does not know we really access VGA buffer memory, so it might decide that these writes are unnecessary and can be omitted.

To avoid those erroneous optimization, we need to specify these writes as [volatile](https://en.wikipedia.org/wiki/Volatile_(computer_programming)). A `volatile` crate is used to wrap the struct.

NOTE to specify `volatile` version as `0.2.6` to make sure it compatible with this project.

```rust
use volatile::Volatile;

struct Buffer {
    chars: [[Volatile<ScreenChar>; BUFFER_WIDTH]; BUFFER_HEIGHT],
}

impl Writer {
   pub fn write_byte(&mut self, byte: u8) {
      // ...
      self.buffer.chars[row][col].write(ScreenChar { // change assign operator to `write` method
         ascii_character: byte,
         color_code,
      });
   }
    
    pub fn new_line(&mut self) {
        // ...
        let character = self.buffer.chars[row][col].read(); // change read operation to `read()` method
        self.buffer.chars[row - 1][col].write(character);
    }
    
    fn clear_row(&mut self, row: unsize) {
        // ...
        self.buffer.chars[row][col].write(blank);
    }
}
```

## Support Formatting Macros

To support Rust's formatting macros, like `write!()` and `writeln!()`, which can help us print different types like integers and floats, we should implement `core::fmt::Write` trait for `Writer`:

```rust
// to impl fmt::Write, the only required method is `write_str`
impl fmt::Write for Writer {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        self.write_string(s);
        Ok(())
    }
}
```

Now the `write!()` and `writeln!()` can be used in combination with our customized `Writer`:

```rust
pub fn print_something() {
    use core::fmt::Write;
    let mut writer = Writer {
        column_position: 0,
        color_code: ColorCode::new(Color::Yellow, Color::Black),
        buffer: unsafe { &mut *(0xb8000 as *mut Buffer) },
    };
    
    write!(write, "Then number are {} and {}", 42, 1.0/3.0).unwrap();
}
```

NOTE that `use core::fmt::Write` trait is required to use `write!` macro, because we only implemented `write_str` method, and there are many other methods needed when use `write!` macro. Those methods are already defined in `core::fmt::Write` trait, and we should include this trait.

## A Global Interface

We can try to create a global instance for `Writer`:

```rust
pub static WRITER: Writer = Writer {
    column_position: 0,
    color_code: ColorCode::new(Color::Yellow, Color::Black),
    buffer: unsafe { &mut *(0xb8000 as *mut Buffer) },
};
```

But this can cause many compile time errors, because the statics are initialized at compile time rather tan at run time like normal variables.

Rust use "const evaluator" to evaluate those initialization expressions, its functionality is limited. For example, we can not call `ColorCode::new` and can not dereference raw pointers in statics.

So `lazy_static` crate is introduced to solve this problem. It provides a `lazy_static!` macro to lazily initialize itself when accessed for the first time instead of at compile time.

Add the crate:

```toml
# in Cargo.toml

[dependencies.lazy_static]
version = "1.0"
# spin_no_std feature is added because we do not link the std library
features = ["spin_no_std"]
```

```rust
// in src/vga_buffer.rs

use lazy_static::lazy_static;

lazy_static! {
    pub static ref WRITER: Writer = Writer {
        column_position: 0,
        color_code: ColorCode::new(Color::Yellow, Color::Black),
        buffer: unsafe { &mut *(0xb8000 as *mut Buffer) },
    };
}
```

But we can not write anything to it, because all writing method depends on `&mut self`, while this static is immutable. `static mut` is highly discouraged, because it introduces data races.

The standard library provides `Mutex`, but we can not use it here since we even have a concept of threads.

We can use a simple spinlock to add safe interior mutability.

Spinlock is a lock that causes a thread trying to acquire it to simply wait in a loop ("spin") while repeatedly checking whether the lock is available. 
