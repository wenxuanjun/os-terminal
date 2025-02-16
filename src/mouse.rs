#[derive(Debug)]
pub enum MouseButton {
    Left,
    Right,
    Middle,
}

#[derive(Debug)]
pub enum MouseInput {
    Move(usize, usize),
    Scroll(isize),
    Pressed(MouseButton),
    Released(MouseButton),
}

#[derive(Debug)]
pub enum MouseEvent {
    Scroll(isize),
    None,
}

pub struct MouseManager {
    scroll_speed: usize,
}

impl Default for MouseManager {
    fn default() -> Self {
        Self {
            scroll_speed: 1,
        }
    }
}

impl MouseManager {
    pub fn set_scroll_speed(&mut self, speed: usize) {
        self.scroll_speed = speed;
    }

    pub fn handle_mouse(&mut self, event: MouseInput) -> MouseEvent {
        match event {
            MouseInput::Scroll(lines) => {
                let lines = lines * self.scroll_speed as isize;
                MouseEvent::Scroll(lines)
            }
            _ => MouseEvent::None,
        }
    }
}
