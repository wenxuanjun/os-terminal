use minifb::{Key, Window, WindowOptions};
use os_terminal::{DrawTarget, Rgb888, Terminal};

use std::io::Read;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;
use std::time::Duration;

const DISPLAY_SIZE: (usize, usize) = (800, 600);

struct Display {
    pub width: usize,
    pub height: usize,
    buffer: Arc<Vec<AtomicU32>>,
}

impl DrawTarget for Display {
    fn size(&self) -> (usize, usize) {
        (self.width, self.height)
    }

    #[inline]
    fn draw_pixel(&mut self, x: usize, y: usize, color: Rgb888) {
        let value = (color.0 as u32) << 16 | (color.1 as u32) << 8 | color.2 as u32;
        self.buffer[y * self.width + x].store(value, Ordering::Relaxed);
    }
}

fn main() {
    let buffer = (0..DISPLAY_SIZE.0 * DISPLAY_SIZE.1)
        .map(|_| AtomicU32::new(0))
        .collect::<Vec<_>>();
    let buffer = Arc::new(buffer);

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
    )
    .unwrap();

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
            let buffer = buffer
                .iter()
                .map(|pixel| pixel.load(Ordering::Relaxed))
                .collect::<Vec<_>>();
            window
                .update_with_buffer(&buffer, DISPLAY_SIZE.0, DISPLAY_SIZE.1)
                .unwrap();
        }
        std::thread::sleep(Duration::from_millis(20));
    }
}
