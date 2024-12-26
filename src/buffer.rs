use alloc::collections::vec_deque::VecDeque;
use alloc::vec::Vec;
use core::mem::swap;

use crate::cell::Cell;
use crate::graphic::{DrawTarget, Graphic};

const INIT_SIZE: (usize, usize) = (1, 1);
const DEFAULT_HISTORY_SIZE: usize = 200;

pub struct FixedStack<T> {
    data: VecDeque<T>,
    capacity: usize,
}

impl<T> FixedStack<T> {
    #[inline]
    pub fn new(capacity: usize) -> Self {
        let data = VecDeque::with_capacity(capacity);
        Self { data, capacity }
    }

    #[inline]
    pub fn len(&self) -> usize {
        self.data.len()
    }

    #[inline]
    pub fn is_empty(&self) -> bool {
        self.data.is_empty()
    }

    #[inline]
    pub fn push(&mut self, item: T) {
        if self.data.len() >= self.capacity {
            self.data.pop_front();
        }
        self.data.push_back(item);
    }

    #[inline]
    pub fn pop(&mut self) -> Option<T> {
        self.data.pop_back()
    }

    #[inline]
    pub fn clear(&mut self) {
        self.data.clear();
    }

    pub fn resize(&mut self, capacity: usize) {
        self.capacity = capacity;
        while self.data.len() > capacity {
            self.data.pop_front();
        }
        self.data.shrink_to(capacity);
    }
}

pub struct TerminalBuffer<D: DrawTarget> {
    graphic: Graphic<D>,
    size: (usize, usize),
    pixel_size: (usize, usize),
    alt_screen_mode: bool,
    flush_cache: VecDeque<Vec<Cell>>,
    buffer: VecDeque<Vec<Cell>>,
    alt_buffer: VecDeque<Vec<Cell>>,
    above_buffer: FixedStack<Vec<Cell>>,
    below_buffer: FixedStack<Vec<Cell>>,
}

impl<D: DrawTarget> TerminalBuffer<D> {
    #[inline]
    pub fn width(&self) -> usize {
        self.size.0
    }

    #[inline]
    pub fn height(&self) -> usize {
        self.size.1
    }
}

impl<D: DrawTarget> TerminalBuffer<D> {
    pub fn new(graphic: Graphic<D>) -> Self {
        let buffer = vec![vec![Cell::default(); INIT_SIZE.0]; INIT_SIZE.1];

        Self {
            graphic,
            size: INIT_SIZE,
            pixel_size: (0, 0),
            alt_screen_mode: false,
            buffer: buffer.clone().into(),
            alt_buffer: buffer.clone().into(),
            flush_cache: buffer.into(),
            above_buffer: FixedStack::new(DEFAULT_HISTORY_SIZE),
            below_buffer: FixedStack::new(DEFAULT_HISTORY_SIZE),
        }
    }

    pub fn swap_alt_screen(&mut self, cell: Cell) {
        self.alt_screen_mode = !self.alt_screen_mode;
        swap(&mut self.buffer, &mut self.alt_buffer);

        if self.alt_screen_mode {
            self.clear(cell);
        }
    }

    pub fn update_size(&mut self, font_width: usize, font_height: usize) {
        if font_width == 0 || font_height == 0 {
            return;
        }

        let width = self.graphic.width() / font_width;
        let height = self.graphic.height() / font_height;
        self.pixel_size = (font_width * width, font_height * height);

        if self.size != (width, height) {
            let buffer = vec![vec![Cell::default(); width]; height].into();
            self.size = (width, height);
            self.buffer.clone_from(&buffer);
            self.alt_buffer.clone_from(&buffer);
            self.flush_cache = buffer;
        }
    }
}

impl<D: DrawTarget> TerminalBuffer<D> {
    #[inline]
    pub fn read(&self, row: usize, col: usize) -> Cell {
        let row = row % self.height();
        self.buffer[row][col]
    }

    #[inline]
    pub fn write(&mut self, row: usize, col: usize, cell: Cell) {
        let row = row % self.height();
        self.buffer[row][col] = cell;
    }

    #[inline]
    pub fn clear(&mut self, cell: Cell) {
        self.buffer
            .iter_mut()
            .flat_map(|row| row.iter_mut())
            .for_each(|c| *c = cell);
    }
}

impl<D: DrawTarget> TerminalBuffer<D> {
    #[inline]
    pub fn flush(&mut self) {
        for (i, row) in self.buffer.iter().enumerate() {
            for (j, &cell) in row.iter().enumerate() {
                if cell != self.flush_cache[i][j] {
                    self.graphic.write(i, j, cell);
                    self.flush_cache[i][j] = cell;
                }
            }
        }
    }

    pub fn full_flush(&mut self) {
        macro_rules! reset_buffer {
            ($buffer:expr) => {
                $buffer
                    .iter_mut()
                    .flat_map(|row| row.iter_mut())
                    .for_each(|c| *c = c.reset_color());
            };
        }

        reset_buffer!(self.buffer);
        reset_buffer!(self.alt_buffer);
        reset_buffer!(self.above_buffer.data);
        reset_buffer!(self.below_buffer.data);

        for (i, row) in self.buffer.iter().enumerate() {
            for (j, &cell) in row.iter().enumerate() {
                self.graphic.write(i, j, cell);
            }
        }

        self.graphic.clear(
            (0, self.pixel_size.1),
            (self.pixel_size.0, self.graphic.height()),
            Cell::default(),
        );
        self.graphic.clear(
            (self.pixel_size.0, 0),
            (self.graphic.width(), self.graphic.height()),
            Cell::default(),
        );
    }
}

impl<D: DrawTarget> TerminalBuffer<D> {
    #[inline]
    pub fn is_latest(&self) -> bool {
        if self.alt_screen_mode {
            true
        } else {
            self.below_buffer.is_empty()
        }
    }

    #[inline]
    pub fn back_to_latest(&mut self) {
        if !self.alt_screen_mode {
            self.scroll_history(self.below_buffer.len(), true);
        }
    }

    #[inline]
    pub fn clear_history(&mut self) {
        if !self.alt_screen_mode {
            self.above_buffer.clear();
            self.below_buffer.clear();
        }
    }

    #[inline]
    pub fn resize_history(&mut self, new_capacity: usize) {
        self.above_buffer.resize(new_capacity);
        self.below_buffer.resize(new_capacity);
    }
}

impl<D: DrawTarget> TerminalBuffer<D> {
    pub fn scroll(
        &mut self,
        count: usize,
        cell: Cell,
        is_up: bool,
        scrolling_region: (usize, usize),
    ) {
        let (top, bottom) = scrolling_region;
        let new_row = vec![cell; self.width()];

        for _ in 0..count {
            if is_up {
                let row = self.buffer.remove(top).unwrap();
                if bottom >= self.height() && !self.alt_screen_mode {
                    self.above_buffer.push(row);
                }
                self.buffer.insert(bottom, new_row.clone());
            } else {
                let row = self.buffer.remove(bottom).unwrap();
                if top <= 1 && !self.alt_screen_mode {
                    self.below_buffer.push(row);
                }
                self.buffer.insert(top, new_row.clone());
            }
        }
    }

    pub fn scroll_history(&mut self, count: usize, is_up: bool) {
        if self.alt_screen_mode {
            return;
        }

        let moves = if is_up {
            count.min(self.below_buffer.len())
        } else {
            count.min(self.above_buffer.len())
        };

        for _ in 0..moves {
            if is_up {
                let row = self.buffer.pop_front().unwrap();
                self.above_buffer.push(row);
                self.buffer.push_back(self.below_buffer.pop().unwrap());
            } else {
                let row = self.buffer.pop_back().unwrap();
                self.below_buffer.push(row);
                self.buffer.push_front(self.above_buffer.pop().unwrap());
            }
        }
    }
}
