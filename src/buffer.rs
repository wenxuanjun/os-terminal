use alloc::collections::vec_deque::VecDeque;
use alloc::vec::Vec;
use core::mem::swap;
use core::ops::Range;
use derive_more::{Deref, DerefMut};

use crate::cell::Cell;
use crate::graphic::{DrawTarget, Graphic};

const INIT_SIZE: (usize, usize) = (1, 1);
const DEFAULT_HISTORY_SIZE: usize = 200;

#[derive(Deref, DerefMut)]
pub struct FixedStack<T> {
    #[deref]
    #[deref_mut]
    data: VecDeque<T>,
    capacity: usize,
}

impl<T> FixedStack<T> {
    pub fn new(capacity: usize) -> Self {
        let data = VecDeque::with_capacity(capacity);
        Self { data, capacity }
    }

    pub fn push(&mut self, item: T) {
        if self.data.len() == self.capacity {
            self.data.pop_front();
        }
        self.data.push_back(item);
    }

    pub fn pop(&mut self) -> Option<T> {
        self.data.pop_back()
    }

    pub fn resize(&mut self, capacity: usize) {
        self.capacity = capacity;
        if self.data.len() > capacity {
            let split_at = self.data.len() - capacity;
            self.data = self.data.split_off(split_at);
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
    pub fn width(&self) -> usize {
        self.size.0
    }

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

        let width = self.graphic.size().0 / font_width;
        let height = self.graphic.size().1 / font_height;
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
    pub fn read(&self, row: usize, col: usize) -> Cell {
        let row = row % self.height();
        self.buffer[row][col]
    }

    pub fn write(&mut self, row: usize, col: usize, cell: Cell) {
        let row = row % self.height();
        self.buffer[row][col] = cell;
    }

    pub fn clear(&mut self, cell: Cell) {
        self.buffer
            .iter_mut()
            .flat_map(|row| row.iter_mut())
            .for_each(|c| *c = cell);
    }
}

impl<D: DrawTarget> TerminalBuffer<D> {
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
        for buffer in &mut [
            &mut self.buffer,
            &mut self.alt_buffer,
            &mut self.above_buffer.data,
            &mut self.below_buffer.data,
        ] {
            buffer
                .iter_mut()
                .flat_map(|row| row.iter_mut())
                .for_each(|c| *c = c.reset_color());
        }

        for (i, row) in self.buffer.iter().enumerate() {
            for (j, &cell) in row.iter().enumerate() {
                self.graphic.write(i, j, cell);
            }
        }

        let color = Cell::default().background.to_rgb();

        for y in self.pixel_size.1..self.graphic.size().1 {
            for x in 0..self.pixel_size.0 {
                self.graphic.draw_pixel(x, y, color);
            }
        }
        for y in 0..self.graphic.size().1 {
            for x in self.pixel_size.0..self.graphic.size().0 {
                self.graphic.draw_pixel(x, y, color);
            }
        }
    }
}

impl<D: DrawTarget> TerminalBuffer<D> {
    pub fn is_latest(&self) -> bool {
        if self.alt_screen_mode {
            true
        } else {
            self.below_buffer.is_empty()
        }
    }

    pub fn goto_latest(&mut self) {
        if !self.alt_screen_mode {
            let len = self.below_buffer.len();
            self.scroll_history(-(len as isize));
        }
    }

    pub fn clear_history(&mut self) {
        if !self.alt_screen_mode {
            self.above_buffer.clear();
            self.below_buffer.clear();
        }
    }

    pub fn resize_history(&mut self, new_capacity: usize) {
        self.above_buffer.resize(new_capacity);
        self.below_buffer.resize(new_capacity);
    }
}

impl<D: DrawTarget> TerminalBuffer<D> {
    pub fn scroll_history(&mut self, count: isize) {
        if self.alt_screen_mode {
            return;
        }

        let moves = if count > 0 {
            count.unsigned_abs().min(self.above_buffer.len())
        } else {
            count.unsigned_abs().min(self.below_buffer.len())
        };

        if count > 0 {
            for _ in 0..moves {
                self.below_buffer.push(self.buffer.pop_back().unwrap());
                self.buffer.push_front(self.above_buffer.pop().unwrap());
            }
        } else {
            for _ in 0..moves {
                self.above_buffer.push(self.buffer.pop_front().unwrap());
                self.buffer.push_back(self.below_buffer.pop().unwrap());
            }
        }
    }

    pub fn scroll_region(&mut self, count: isize, cell: Cell, region: Range<usize>) {
        let (top, bottom) = (region.start, region.end);

        if count > 0 {
            for _ in 0..count.unsigned_abs() {
                let row = self.buffer.remove(bottom).unwrap();
                if !self.alt_screen_mode && top == 0 {
                    self.below_buffer.push(row);
                }
                self.buffer.insert(top, vec![cell; self.width()]);
            }
        } else {
            for _ in 0..count.unsigned_abs() {
                let row = self.buffer.remove(top).unwrap();
                if !self.alt_screen_mode && bottom == self.height() - 1 {
                    self.above_buffer.push(row);
                }
                self.buffer.insert(bottom, vec![cell; self.width()]);
            }
        }
    }
}
