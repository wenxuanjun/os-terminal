use std::ffi::CString;
use std::num::NonZeroU32;
use std::os::fd::AsFd;
use std::os::unix::io::AsRawFd;
use std::process;
use std::rc::Rc;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::{Arc, Mutex};

use crossbeam_channel::{unbounded, Sender};
use keycode::{KeyMap, KeyMapping};
use nix::errno::Errno;
use nix::libc::{ioctl, TIOCSWINSZ};
use nix::pty::{openpty, OpenptyResult, Winsize};
use nix::unistd::{close, dup2, execvp, fork, read, setsid, write, ForkResult};
use os_terminal::font::TrueTypeFont;
use os_terminal::{DrawTarget, Rgb888, Terminal};

use softbuffer::{Context, Surface};
use winit::application::ApplicationHandler;
use winit::dpi::PhysicalSize;
use winit::event::{ElementState, WindowEvent};
use winit::event_loop::{ActiveEventLoop, EventLoop};
use winit::platform::scancode::PhysicalKeyExtScancode;
use winit::window::{Window, WindowAttributes, WindowId};

const DISPLAY_SIZE: (usize, usize) = (1024, 768);

fn main() {
    let display = Display::default();
    let buffer = display.buffer.clone();

    let terminal = {
        let mut terminal = Terminal::new(display);
        terminal.set_auto_flush(false);
        terminal.set_logger(Some(|args| println!("Terminal: {:?}", args)));

        let font_buffer = include_bytes!("SourceCodeVF.otf");
        terminal.set_font_manager(Box::new(TrueTypeFont::new(10.0, font_buffer)));

        Arc::new(Mutex::new(terminal))
    };

    let OpenptyResult { master, slave } = openpty(None, None).unwrap();

    match unsafe { fork() } {
        Ok(ForkResult::Child) => {
            // In the child process, we execute bash
            close(master.as_raw_fd()).unwrap();

            let win_size = {
                let terminal = terminal.lock().unwrap();
                Winsize {
                    ws_row: terminal.rows() as u16,
                    ws_col: terminal.columns() as u16,
                    ws_xpixel: DISPLAY_SIZE.0 as u16,
                    ws_ypixel: DISPLAY_SIZE.1 as u16,
                }
            };

            setsid().unwrap();
            unsafe { ioctl(slave.as_raw_fd(), TIOCSWINSZ, &win_size) };

            dup2(slave.as_raw_fd(), 0).unwrap();
            dup2(slave.as_raw_fd(), 1).unwrap();
            dup2(slave.as_raw_fd(), 2).unwrap();

            let _ = execvp::<CString>(&CString::new("bash").unwrap(), &[]);
        }
        Ok(ForkResult::Parent { .. }) => {
            // In the parent process, we handle the terminal I/O
            close(slave.as_raw_fd()).unwrap();
            let master_raw_fd = master.as_raw_fd();

            let (ansi_sender, ansi_receiver) = unbounded();
            let mut app = App::new(ansi_sender, buffer.clone(), terminal.clone());

            let event_loop = EventLoop::new().unwrap();
            let redraw_event_proxy = event_loop.create_proxy();

            std::thread::spawn(move || {
                let mut temp = [0u8; 1024];
                loop {
                    match read(master_raw_fd, &mut temp) {
                        Ok(n) if n > 0 => {
                            terminal.lock().unwrap().advance_state(&temp[..n]);
                            redraw_event_proxy.send_event(()).unwrap();
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
                while let Ok(key) = ansi_receiver.recv() {
                    write(master.as_fd(), key.as_bytes()).unwrap();
                }
            });

            event_loop.run_app(&mut app).unwrap();
        }
        Err(_) => eprintln!("Fork failed"),
    }
}

struct Display {
    width: usize,
    height: usize,
    buffer: Arc<Vec<AtomicU32>>,
}

impl Default for Display {
    fn default() -> Self {
        let buffer = (0..DISPLAY_SIZE.0 * DISPLAY_SIZE.1)
            .map(|_| AtomicU32::new(0))
            .collect::<Vec<_>>();

        Self {
            width: DISPLAY_SIZE.0,
            height: DISPLAY_SIZE.1,
            buffer: Arc::new(buffer),
        }
    }
}

impl DrawTarget for Display {
    fn size(&self) -> (usize, usize) {
        (self.width, self.height)
    }

    #[inline(always)]
    fn draw_pixel(&mut self, x: usize, y: usize, color: Rgb888) {
        let value = (color.0 as u32) << 16 | (color.1 as u32) << 8 | color.2 as u32;
        self.buffer[y * self.width + x].store(value, Ordering::Relaxed);
    }
}

struct App {
    ansi_sender: Sender<String>,
    buffer: Arc<Vec<AtomicU32>>,
    terminal: Arc<Mutex<Terminal<Display>>>,
    window: Option<Rc<Window>>,
    surface: Option<Surface<Rc<Window>, Rc<Window>>>,
}

impl App {
    fn new(
        ansi_sender: Sender<String>,
        buffer: Arc<Vec<AtomicU32>>,
        terminal: Arc<Mutex<Terminal<Display>>>,
    ) -> Self {
        Self {
            ansi_sender,
            buffer,
            terminal,
            window: None,
            surface: None,
        }
    }
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        let (width, height) = DISPLAY_SIZE;
        let attributes = WindowAttributes::default()
            .with_title("Terminal")
            .with_resizable(false)
            .with_inner_size(PhysicalSize::new(width as f64, height as f64));
        let window = Rc::new(event_loop.create_window(attributes).unwrap());

        let context = Context::new(window.clone()).unwrap();
        let mut surface = Surface::new(&context, window.clone()).unwrap();

        surface
            .resize(
                NonZeroU32::new(width as u32).unwrap(),
                NonZeroU32::new(height as u32).unwrap(),
            )
            .unwrap();

        self.window = Some(window);
        self.surface = Some(surface);
    }

    fn user_event(&mut self, _event_loop: &ActiveEventLoop, _event: ()) {
        if let Some(window) = self.window.as_ref() {
            window.request_redraw();
        }
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        window_id: WindowId,
        event: WindowEvent,
    ) {
        let window = self.window.as_ref().unwrap();

        match event {
            WindowEvent::RedrawRequested => {
                if window_id == window.id() {
                    let surface = self.surface.as_mut().unwrap();
                    self.terminal.lock().unwrap().flush();

                    let buffer = self
                        .buffer
                        .iter()
                        .map(|pixel| pixel.load(Ordering::Relaxed))
                        .collect::<Vec<_>>();

                    let mut surface_buffer = surface.buffer_mut().unwrap();
                    surface_buffer.copy_from_slice(&buffer[..]);
                    surface_buffer.present().unwrap();
                }
            }
            WindowEvent::CloseRequested => {
                if window_id == window.id() {
                    event_loop.exit();
                }
            }
            WindowEvent::KeyboardInput { event, .. } => {
                if window_id == window.id() {
                    if let Some(evdev_code) = event.physical_key.to_scancode() {
                        if let Ok(keymap) =
                            KeyMap::from_key_mapping(KeyMapping::Evdev(evdev_code as u16))
                        {
                            // Windows scancode is 16-bit extended scancode
                            let mut scancode = keymap.win;
                            if event.state == ElementState::Released {
                                scancode += 0x80;
                            }
                            if scancode >= 0xe000 {
                                self.terminal.lock().unwrap().handle_keyboard(0xe0);
                                scancode -= 0xe000;
                            }
                            if let Some(ansi_string) = self
                                .terminal
                                .lock()
                                .unwrap()
                                .handle_keyboard(scancode as u8)
                            {
                                self.ansi_sender.send(ansi_string).unwrap();
                            }
                        }
                    }
                }
            }
            _ => {}
        }
    }
}
