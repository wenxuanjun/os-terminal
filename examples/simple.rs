use os_terminal::{Terminal, DrawTarget};
use minifb::{Key, Window, WindowOptions};

use std::io::Read;
use std::time::Duration;
use std::sync::{Arc, RwLock};

const DISPLAY_SIZE: (usize, usize) = (800, 600);

struct Display {
    pub width: usize,
    pub height: usize,
    pub buffer: Arc<RwLock<Vec<u32>>>,
}

impl DrawTarget for Display {
    fn size(&self) -> (usize, usize) {
        (self.width, self.height)
    }

    fn draw_pixel(&mut self, x: usize, y: usize, color: (u8, u8, u8)) {
        let value = (color.0 as u32) << 16 | (color.1 as u32) << 8 | color.2 as u32;
        let mut buffer = self.buffer.write().unwrap();
        buffer[y * self.width + x] = value;
    }
}

fn main() {
    env_logger::init();

    let buffer = vec![0; DISPLAY_SIZE.0 * DISPLAY_SIZE.1];
    let buffer = Arc::new(RwLock::new(buffer));

    let display = Display {
        width: DISPLAY_SIZE.0,
        height: DISPLAY_SIZE.1,
        buffer: buffer.clone(),
    };

    let mut window = Window::new(
        "Test",
        DISPLAY_SIZE.0,
        DISPLAY_SIZE.1,
        WindowOptions::default(),
    ).unwrap();

    let mut terminal = Terminal::new(display);

    std::thread::spawn(move || {
        for c in std::io::stdin().lock().bytes() {
            let c = c.unwrap();
            if c == 0xff {
                break;
            }
            terminal.write_bstr(&[c]);
        }
    });

    while window.is_open() && !window.is_key_down(Key::Escape) {
        {
            let buffer = buffer.read().unwrap();
            window.update_with_buffer(&buffer, DISPLAY_SIZE.0, DISPLAY_SIZE.1).unwrap();
        }
        std::thread::sleep(Duration::from_millis(20));
    }
}
