# OS Terminal

A `no_std` terminal library for embedded systems and OS kernels.

## Features

- Embedded smooth noto font rendering
- VT100 and part of xterm escape sequence support
- Beautiful color scheme

## Usage

Create a display wrapper to wrap your framebuffer and implement the `DrawTarget` trait for it.

```rust
struct Display {
    width: usize,
    height: usize,
    buffer: &'static [u32],
}

impl DrawTarget for Display {
    fn size(&self) -> (usize, usize) {
        (self.width, self.height)
    }

    fn draw_pixel(&mut self, x: usize, y: usize, color: (u8, u8, u8)) {
        let value = (color.0 as u32) << 16 | (color.1 as u32) << 8 | color.2 as u32;
        self.buffer[y * self.width + x] = value;
    }
}
```

Then you can create a console and write to it.

```rust
let mut console = Console::new(display);
console.write_fmt(format_args!("Hello, world"));
console.write_byte(b'!');
```

## Acknowledgement

- [embedded-term](https://github.com/rcore-os/embedded-term): This project is a fork of it with some simplifications and improvements.
- [alacritty](https://github.com/CyberFlameGO/alacritty): General reference for the terminal implementation.
- [noto-sans-mono-bitmap-rs](https://github.com/phip1611/noto-sans-mono-bitmap-rs): Pre-rasterized smooth characters.

Thanks to the original author and contributors for their great work.
