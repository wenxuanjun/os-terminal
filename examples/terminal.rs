use std::error::Error;
use std::ffi::CString;
use std::num::NonZeroU32;
use std::os::fd::AsFd;
use std::os::unix::io::{AsRawFd, IntoRawFd};
use std::rc::Rc;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::mpsc::{channel, Sender};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use std::{env, process};

use keycode::{KeyMap, KeyMapping};
use nix::libc::{ioctl, TIOCSWINSZ};
use nix::pty::{openpty, OpenptyResult, Winsize};
use nix::unistd::{close, execvp, fork, read, write, ForkResult};
use nix::unistd::{dup2_stderr, dup2_stdin, dup2_stdout, setsid};
use os_terminal::font::TrueTypeFont;
use os_terminal::{ClipboardHandler, DrawTarget, MouseInput, Rgb, Terminal};

use softbuffer::{Context, Surface};
use winit::application::ApplicationHandler;
use winit::dpi::PhysicalSize;
use winit::event::{ElementState, Ime, MouseScrollDelta, StartCause, WindowEvent};
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};
use winit::platform::scancode::PhysicalKeyExtScancode;
use winit::window::{ImePurpose, Window, WindowAttributes, WindowId};

const DISPLAY_SIZE: (usize, usize) = (1024, 768);
const TOUCHPAD_SCROLL_MULTIPLIER: f32 = 0.25;

struct Clipboard(arboard::Clipboard);

impl Clipboard {
    fn new() -> Self {
        Self(arboard::Clipboard::new().unwrap())
    }
}

impl ClipboardHandler for Clipboard {
    fn get_text(&mut self) -> Option<String> {
        self.0.get_text().ok()
    }

    fn set_text(&mut self, text: String) {
        self.0.set_text(text).unwrap();
    }
}

fn main() -> Result<(), Box<dyn Error>> {
    let OpenptyResult { master, slave } = openpty(None, None)?;

    match unsafe { fork() } {
        Ok(ForkResult::Child) => {
            close(master.into_raw_fd())?;

            setsid()?;
            dup2_stdin(slave.as_fd())?;
            dup2_stdout(slave.as_fd())?;
            dup2_stderr(slave.as_fd())?;

            let shell = env::var("SHELL").unwrap_or("bash".into());
            let _ = execvp::<CString>(&CString::new(shell)?, &[]);
        }
        Ok(ForkResult::Parent { .. }) => {
            close(slave.into_raw_fd())?;

            let display = Display::default();
            let buffer = display.buffer.clone();
            let (ansi_sender, ansi_receiver) = channel();

            let mut terminal = Terminal::new(display);
            terminal.set_auto_flush(false);
            terminal.set_scroll_speed(5);
            terminal.set_logger(|args| println!("Terminal: {:?}", args));
            terminal.set_clipboard(Box::new(Clipboard::new()));

            terminal.set_pty_writer({
                let ansi_sender = ansi_sender.clone();
                Box::new(move |data| ansi_sender.send(data).unwrap())
            });

            let font_buffer = include_bytes!("FiraCodeNotoSans.ttf");
            terminal.set_font_manager(Box::new(TrueTypeFont::new(10.0, font_buffer)));
            terminal.set_history_size(1000);

            let win_size = Winsize {
                ws_row: terminal.rows() as u16,
                ws_col: terminal.columns() as u16,
                ws_xpixel: DISPLAY_SIZE.0 as u16,
                ws_ypixel: DISPLAY_SIZE.1 as u16,
            };

            unsafe { ioctl(master.as_raw_fd(), TIOCSWINSZ, &win_size) };

            let event_loop = EventLoop::new()?;
            let terminal = Arc::new(Mutex::new(terminal));
            let pending_draw = Arc::new(AtomicBool::new(false));

            let mut app = App::new(
                ansi_sender,
                buffer.clone(),
                terminal.clone(),
                pending_draw.clone(),
            );

            let master_clone = master.try_clone()?;
            std::thread::spawn(move || {
                let mut temp = [0u8; 4096];
                loop {
                    match read(master_clone.as_fd(), &mut temp) {
                        Ok(n) if n > 0 => {
                            terminal.lock().unwrap().process(&temp[..n]);
                            pending_draw.store(true, Ordering::Relaxed);
                        }
                        Ok(_) => break,
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

            event_loop.run_app(&mut app)?;
        }
        Err(_) => eprintln!("Fork failed"),
    }

    Ok(())
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
        let color = (color.0 as u32) << 16 | (color.1 as u32) << 8 | color.2 as u32;
        self.buffer[y * self.width + x].store(color, Ordering::Relaxed);
    }
}

struct App {
    ansi_sender: Sender<String>,
    buffer: Arc<Vec<AtomicU32>>,
    terminal: Arc<Mutex<Terminal<Display>>>,
    window: Option<Rc<Window>>,
    surface: Option<Surface<Rc<Window>, Rc<Window>>>,
    pending_draw: Arc<AtomicBool>,
    scroll_accumulator: f32,
}

impl App {
    fn new(
        ansi_sender: Sender<String>,
        buffer: Arc<Vec<AtomicU32>>,
        terminal: Arc<Mutex<Terminal<Display>>>,
        pending_draw: Arc<AtomicBool>,
    ) -> Self {
        Self {
            ansi_sender,
            buffer,
            terminal,
            window: None,
            surface: None,
            pending_draw,
            scroll_accumulator: 0.0,
        }
    }
}

impl ApplicationHandler for App {
    fn new_events(&mut self, _: &ActiveEventLoop, cause: StartCause) {
        if !matches!(cause, StartCause::ResumeTimeReached { .. })
            || !self.pending_draw.swap(false, Ordering::Relaxed)
        {
            return;
        }
        if let Some(surface) = self.surface.as_mut() {
            self.terminal.lock().unwrap().flush();

            let mut buffer = surface.buffer_mut().unwrap();
            for (index, value) in self.buffer.iter().enumerate() {
                buffer[index] = value.load(Ordering::Relaxed);
            }

            buffer.present().unwrap();
        }
    }

    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        let refresh_rate = event_loop
            .primary_monitor()
            .and_then(|m| m.refresh_rate_millihertz())
            .unwrap_or(60000);

        let frame_duration = 1000.0 / (refresh_rate as f32 / 1000.0);
        let duration = Duration::from_millis(frame_duration as u64);
        event_loop.set_control_flow(ControlFlow::WaitUntil(Instant::now() + duration));
    }

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

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _: WindowId, event: WindowEvent) {
        match event {
            WindowEvent::CloseRequested => {
                event_loop.exit();
            }
            WindowEvent::Ime(Ime::Commit(text)) => {
                self.ansi_sender.send(text).unwrap();
            }
            WindowEvent::MouseWheel { delta, .. } => {
                self.scroll_accumulator += match delta {
                    MouseScrollDelta::LineDelta(_, lines) => lines,
                    MouseScrollDelta::PixelDelta(delta) => {
                        delta.y as f32 * TOUCHPAD_SCROLL_MULTIPLIER
                    }
                };
                if self.scroll_accumulator.abs() >= 1.0 {
                    let lines = self.scroll_accumulator as isize;
                    self.scroll_accumulator -= lines as f32;
                    self.terminal
                        .lock()
                        .unwrap()
                        .handle_mouse(MouseInput::Scroll(lines));
                    self.pending_draw.store(true, Ordering::Relaxed);
                }
            }
            WindowEvent::KeyboardInput { event, .. } => {
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
                        self.terminal
                            .lock()
                            .unwrap()
                            .handle_keyboard(scancode as u8);
                        self.pending_draw.store(true, Ordering::Relaxed);
                    }
                }
            }
            _ => {}
        }
    }
}
