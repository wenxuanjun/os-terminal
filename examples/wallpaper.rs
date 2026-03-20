use std::cell::RefCell;
use std::error::Error;
use std::rc::Rc;

use os_terminal::font::BitmapFont;
use os_terminal::{DrawTarget, Rgb, Terminal};

const DISPLAY_SIZE: (usize, usize) = (800, 600);
const WALLPAPER: &[u8] = include_bytes!("../screenshot.png");

struct Display {
    width: usize,
    height: usize,
    pixels: Rc<RefCell<Vec<u32>>>,
}

impl Display {
    fn new(width: usize, height: usize) -> Self {
        let pixels = vec![0; width * height];
        Self {
            width,
            height,
            pixels: Rc::new(RefCell::new(pixels)),
        }
    }
}

impl DrawTarget for Display {
    fn size(&self) -> (usize, usize) {
        (self.width, self.height)
    }

    fn draw_pixel(&mut self, x: usize, y: usize, rgb: Rgb) {
        let pixel = (rgb.0 as u32) << 16 | (rgb.1 as u32) << 8 | rgb.2 as u32;
        self.pixels.borrow_mut()[y * self.width + x] = pixel;
    }
}

fn main() -> Result<(), Box<dyn Error>> {
    let display = Display::new(DISPLAY_SIZE.0, DISPLAY_SIZE.1);
    let pixels = display.pixels.clone();

    let mut terminal = Terminal::new(display, Box::new(BitmapFont));
    terminal.set_wallpaper(WALLPAPER)?;
    terminal.process(b"\x1b[1;37mWallpaper enabled via include_bytes!\x1b[0m");
    terminal.flush();

    println!(
        "Rendered wallpaper example into an in-memory framebuffer of {} pixels.",
        pixels.borrow().len()
    );
    println!(
        "Replace `../screenshot.png` with your own PNG and reuse `terminal.set_wallpaper(...)`."
    );

    Ok(())
}
