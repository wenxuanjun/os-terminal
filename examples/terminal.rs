use std::env;
use std::ffi::CString;
use std::num::NonZeroU32;
use std::os::fd::AsFd;
use std::os::unix::io::{AsRawFd, IntoRawFd};
use std::process;
use std::rc::Rc;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::{Arc, Mutex};

use crossbeam_channel::{unbounded, Sender};
use keycode::{KeyMap, KeyMapping};
use nix::errno::Errno;
use nix::libc::{ioctl, TIOCSCTTY, TIOCSWINSZ};
use nix::pty::{openpty, OpenptyResult, Winsize};
use nix::unistd::{close, dup2, execvp, fork, read, setsid, write, ForkResult};
use os_terminal::font::TrueTypeFont;
use os_terminal::{DrawTarget, Rgb, Terminal};

use softbuffer::{Context, Surface};
use winit::application::ApplicationHandler;
use winit::dpi::PhysicalSize;
use winit::event::{ElementState, Ime, WindowEvent};
use winit::event_loop::{ActiveEventLoop, EventLoop, EventLoopProxy};
use winit::platform::scancode::PhysicalKeyExtScancode;
use winit::window::{ImePurpose, Window, WindowAttributes, WindowId};

const DISPLAY_SIZE: (usize, usize) = (1024, 768);

fn main() {
    let display = Display::default();
    let buffer = display.buffer.clone();

    let terminal = {
        let mut terminal = Terminal::new(display);
        terminal.set_auto_flush(false);
        terminal.set_logger(Some(|args| println!("Terminal: {:?}", args)));

        let font_buffer = include_bytes!("FiraCodeNotoSans.ttf");
        terminal.set_font_manager(Box::new(TrueTypeFont::new(10.0, font_buffer)));
        terminal.set_history_size(1000);

        Arc::new(Mutex::new(terminal))
    };

    let OpenptyResult { master, slave } = openpty(None, None).unwrap();

    match unsafe { fork() } {
        Ok(ForkResult::Child) => {
            close(master.into_raw_fd()).unwrap();

            let win_size = {
                let terminal = terminal.lock().unwrap();
                Winsize {
                    ws_row: terminal.rows() as u16,
                    ws_col: terminal.columns() as u16,
                    ws_xpixel: DISPLAY_SIZE.0 as u16,
                    ws_ypixel: DISPLAY_SIZE.1 as u16,
                }
            };

            unsafe {
                setsid().unwrap();
                ioctl(slave.as_raw_fd(), TIOCSCTTY, 0);
                ioctl(slave.as_raw_fd(), TIOCSWINSZ, &win_size);
            }

            dup2(slave.as_raw_fd(), 0).unwrap();
            dup2(slave.as_raw_fd(), 1).unwrap();
            dup2(slave.as_raw_fd(), 2).unwrap();

            let shell = env::var("SHELL").unwrap_or("bash".into());
            let _ = execvp::<CString>(&CString::new(shell).unwrap(), &[]);
        }
        Ok(ForkResult::Parent { .. }) => {
            close(slave.into_raw_fd()).unwrap();

            let event_loop = EventLoop::new().unwrap();
            let redraw_event_proxy = event_loop.create_proxy();
            let (ansi_sender, ansi_receiver) = unbounded();

            let mut app = App::new(
                ansi_sender,
                buffer.clone(),
                terminal.clone(),
                redraw_event_proxy.clone(),
            );

            let master_raw_fd = master.as_raw_fd();

            std::thread::spawn(move || {
                let mut temp = [0u8; 1024];
                loop {
                    match read(master_raw_fd, &mut temp) {
                        Ok(n) if n > 0 => {
                            terminal.lock().unwrap().process(&temp[..n]);
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
    fn draw_pixel(&mut self, x: usize, y: usize, color: Rgb) {
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
    redraw_event_proxy: EventLoopProxy<()>,
}

impl App {
    fn new(
        ansi_sender: Sender<String>,
        buffer: Arc<Vec<AtomicU32>>,
        terminal: Arc<Mutex<Terminal<Display>>>,
        redraw_event_proxy: EventLoopProxy<()>,
    ) -> Self {
        Self {
            ansi_sender,
            buffer,
            terminal,
            window: None,
            surface: None,
            redraw_event_proxy,
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
        window.set_ime_allowed(true);
        window.set_ime_purpose(ImePurpose::Terminal);

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

                    let mut surface_buffer = surface.buffer_mut().unwrap();
                    for (index, value) in self.buffer.iter().enumerate() {
                        surface_buffer[index] = value.load(Ordering::Relaxed);
                    }
                    surface_buffer.present().unwrap();
                }
            }
            WindowEvent::CloseRequested => {
                if window_id == window.id() {
                    event_loop.exit();
                }
            }
            WindowEvent::Ime(ime) => {
                if window_id == window.id() {
                    match ime {
                        Ime::Commit(text) => {
                            self.ansi_sender.send(text).unwrap();
                            self.redraw_event_proxy.send_event(()).unwrap();
                        }
                        _ => {}
                    }
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

                            self.redraw_event_proxy.send_event(()).unwrap();
                        }
                    }
                }
            }
            _ => {}
        }
    }
}
