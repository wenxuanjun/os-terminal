#[derive(Debug)]
pub enum MouseEvent {
    Scroll(isize),
    None,
}

#[derive(Debug)]
pub enum MouseButton {
    Left,
    Right,
    Middle,
}

#[derive(Debug)]
pub enum MouseInput {
    Moved(i16, i16),
    Scroll(f32),
    Pressed(MouseButton),
    Released(MouseButton),
}

pub struct MouseManager {
    scroll_speed: f32,
    natural_scroll: bool,
    scroll_accumulator: f32,
}

impl Default for MouseManager {
    fn default() -> Self {
        Self {
            scroll_speed: 1.0,
            natural_scroll: true,
            scroll_accumulator: 0.0,
        }
    }
}

impl MouseManager {
    pub fn set_scroll_speed(&mut self, speed: f32) {
        self.scroll_speed = speed;
    }

    pub fn set_natural_scroll(&mut self, mode: bool) {
        self.natural_scroll = mode;
    }

    pub fn handle_mouse(&mut self, event: MouseInput) -> MouseEvent {
        match event {
            MouseInput::Scroll(lines) => {
                if lines * self.scroll_accumulator < 0.0 {
                    self.scroll_accumulator = 0.0;
                }
                
                self.scroll_accumulator += lines * self.scroll_speed;
                
                if self.scroll_accumulator.abs() < 1.0 {
                    return MouseEvent::None;
                }
                
                let scroll = self.scroll_accumulator as isize;
                self.scroll_accumulator -= scroll as f32;
                
                MouseEvent::Scroll(if self.natural_scroll {-scroll} else {scroll})
            }
            _ => MouseEvent::None,
        }
    }
}
