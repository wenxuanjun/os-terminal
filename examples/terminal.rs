use crossbeam_channel::{unbounded, Sender};
use minifb::{InputCallback, Key, Window, WindowOptions};
use nix::errno::Errno;
use nix::libc::{ioctl, TIOCSWINSZ};
use nix::pty::{openpty, OpenptyResult, Winsize};
use nix::sys::termios;
use nix::unistd::{close, dup2, execvp, fork, read, setsid, write, ForkResult};
use os_terminal::{DrawTarget, Rgb888, Terminal};

use std::ffi::CString;
use std::os::fd::AsFd;
use std::os::unix::io::AsRawFd;
use std::process;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;
use std::time::Duration;

const DISPLAY_SIZE: (usize, usize) = (1024, 768);

struct KeyboardHandler {
    sender: Sender<u32>,
}

impl KeyboardHandler {
    pub fn new(sender: Sender<u32>) -> KeyboardHandler {
        Self { sender }
    }
}

impl InputCallback for KeyboardHandler {
    fn add_char(&mut self, uni_char: u32) {
        self.sender.try_send(uni_char).unwrap();
    }

    fn set_key_state(&mut self, key: Key, state: bool) {
        if state {
            let key_ascii = match key {
                Key::F1 => "\x1bOP",
                Key::F2 => "\x1bOQ",
                Key::F3 => "\x1bOR",
                Key::F4 => "\x1bOS",
                Key::F5 => "\x1b[15~",
                Key::F6 => "\x1b[17~",
                Key::F7 => "\x1b[18~",
                Key::F8 => "\x1b[19~",
                Key::F9 => "\x1b[20~",
                Key::F10 => "\x1b[21~",
                Key::F11 => "\x1b[23~",
                Key::F12 => "\x1b[24~",
                Key::Up => "\x1b[A",
                Key::Down => "\x1b[B",
                Key::Right => "\x1b[C",
                Key::Left => "\x1b[D",
                Key::Home => "\x1b[H",
                Key::End => "\x1b[F",
                Key::PageUp => "\x1b[5~",
                Key::PageDown => "\x1b[6~",
                _ => "",
            };

            let key_slice_u32 = key_ascii
                .as_bytes()
                .iter()
                .map(|&b| b as u32)
                .collect::<Vec<_>>();

            for key in key_slice_u32 {
                self.sender.try_send(key).unwrap();
            }
        }
    }
}

struct Display {
    width: usize,
    height: usize,
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
        "Terminal",
        DISPLAY_SIZE.0,
        DISPLAY_SIZE.1,
        WindowOptions::default(),
    )
    .unwrap();

    let (key_sender, key_receiver) = unbounded();
    let keyboard_handler = KeyboardHandler::new(key_sender);
    window.set_input_callback(Box::new(keyboard_handler));

    let mut terminal = Terminal::new(display);
    os_terminal::set_logger(|args| println!("Terminal: {:?}", args));

    let OpenptyResult { master, slave } = openpty(None, None).unwrap();

    match unsafe { fork() } {
        Ok(ForkResult::Child) => {
            // In the child process, we execute bash
            close(master.as_raw_fd()).unwrap();

            let win_size = Winsize {
                ws_row: terminal.rows() as u16,
                ws_col: terminal.columns() as u16,
                ws_xpixel: DISPLAY_SIZE.0 as u16,
                ws_ypixel: DISPLAY_SIZE.1 as u16,
            };

            setsid().unwrap();
            unsafe { ioctl(slave.as_raw_fd(), TIOCSWINSZ, &win_size) };

            dup2(slave.as_raw_fd(), 0).unwrap();
            dup2(slave.as_raw_fd(), 1).unwrap();
            dup2(slave.as_raw_fd(), 2).unwrap();

            let termios = termios::tcgetattr(slave.as_fd()).unwrap();
            termios::tcsetattr(slave, termios::SetArg::TCSANOW, &termios).unwrap();

            let _ = execvp(
                &CString::new("bash").unwrap(),
                &[CString::new("bash").unwrap()],
            );
        }
        Ok(ForkResult::Parent { .. }) => {
            // In the parent process, we handle the terminal I/O
            close(slave.as_raw_fd()).unwrap();
            let master_raw_fd = master.as_raw_fd();

            std::thread::spawn(move || {
                let mut temp = [0u8; 1024];
                loop {
                    match read(master_raw_fd, &mut temp) {
                        Ok(n) if n > 0 => {
                            terminal.write_bstr(&temp[..n]);
                        }
                        Ok(_) => break,
                        Err(Errno::EIO) => process::exit(0),
                        Err(e) => {
                            eprintln!("Error reading from PTY: {:?}", e);
                            process::exit(1)
                        }
                    }
                }
            });

            std::thread::spawn(move || {
                while let Ok(key) = key_receiver.recv() {
                    write(master.as_fd(), &[key as u8]).unwrap();
                }
            });

            while window.is_open() {
                {
                    let buffer = buffer
                        .iter()
                        .map(|pixel| pixel.load(Ordering::Relaxed))
                        .collect::<Vec<_>>();
                    window
                        .update_with_buffer(&buffer, DISPLAY_SIZE.0, DISPLAY_SIZE.1)
                        .unwrap();
                }

                std::thread::sleep(Duration::from_millis(10));
            }
        }
        Err(_) => eprintln!("Fork failed"),
    }
}
