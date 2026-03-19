#![no_main]
use arbitrary::Arbitrary;
use os_terminal::font::{ContentInfo, FontManager, Rasterized};
use os_terminal::{DrawTarget, KeyboardEvent, Rgb, Terminal};
use std::boxed::Box;

#[derive(Debug, Arbitrary)]
enum FuzzOp {
    Input(Vec<u8>),
    FontResize(bool),
    Scroll(bool),
    Flush,
}

type Size = (usize, usize);

libfuzzer_sys::fuzz_target!(|ops: Vec<FuzzOp>| {
    let display = DummyDisplay {
        width: 405,
        height: 300,
    };
    let (mut font_width, mut font_height) = (8, 16);
    let font = DummyFont::new(font_width, font_height);

    let mut term = Terminal::new(display, font);
    term.set_auto_flush(false);

    for op in &ops {
        match op {
            FuzzOp::Input(data) => term.process(data),
            FuzzOp::Scroll(up) => {
                let key = if *up { 0x48 } else { 0x50 };
                term.handle_keyboard(0x1D); // LCtrl down
                term.handle_keyboard(0x2A); // LShift down
                term.handle_keyboard(0xE0); // extended prefix
                term.handle_keyboard(key); // ArrowUp or ArrowDown
                term.handle_keyboard(0xE0);
                term.handle_keyboard(key | 0x80);
                term.handle_keyboard(0xAA); // LShift up
                term.handle_keyboard(0x9D); // LCtrl up
            }
            FuzzOp::FontResize(increase) => {
                let key = if *increase { 0x0D } else { 0x0C };
                term.handle_keyboard(0x1D); // LCtrl down

                let event = term.handle_keyboard(key); // '=' or '-'
                let calc_new_font = || -> Option<(usize, usize)> {
                    let delta = match event? {
                        KeyboardEvent::FontSize(d) => d,
                        _ => return None,
                    };
                    DummyFont::next_size((font_width, font_height), delta)
                };

                if let Some((width, height)) = calc_new_font() {
                    font_width = width;
                    font_height = height;
                    term.set_font_manager(DummyFont::new(font_width, font_height));
                }

                term.handle_keyboard(key | 0x80);
                term.handle_keyboard(0x9D); // LCtrl up
            }
            FuzzOp::Flush => term.flush(),
        }
    }
    term.flush();
});

struct DummyFont {
    raster: Vec<Vec<u8>>,
    width: usize,
    height: usize,
}

impl DummyFont {
    fn new(width: usize, height: usize) -> Box<Self> {
        Box::new(Self {
            raster: Vec::new(),
            width,
            height,
        })
    }

    fn next_size((width, height): Size, delta: isize) -> Option<Size> {
        let new_width = (width as isize + delta).clamp(4, 40) as usize;
        let new_height = (height as isize + delta * 2).clamp(8, 80) as usize;

        if new_width != width || new_height != height {
            Some((new_width, new_height))
        } else {
            None
        }
    }
}

impl FontManager for DummyFont {
    fn size(&self) -> (usize, usize) {
        (self.width, self.height)
    }

    fn rasterize(&mut self, _: ContentInfo) -> Rasterized<'_> {
        Rasterized::GrayVec(&self.raster)
    }
}

struct DummyDisplay {
    width: usize,
    height: usize,
}

impl DrawTarget for DummyDisplay {
    fn size(&self) -> (usize, usize) {
        (self.width, self.height)
    }

    fn draw_pixel(&mut self, x: usize, y: usize, _color: Rgb) {
        if x >= self.width || y >= self.height {
            panic!("Draw out of bounds: ({}, {})", x, y);
        }
    }
}
