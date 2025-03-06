use alloc::collections::vec_deque::VecDeque;
use alloc::vec::Vec;
use core::mem::swap;
use core::ops::Range;

use crate::cell::Cell;
use crate::color::ToRgb;
use crate::graphic::{DrawTarget, Graphic};

const INIT_SIZE: (usize, usize) = (1, 1);
const DEFAULT_HISTORY_SIZE: usize = 200;

pub struct TerminalBuffer<D: DrawTarget> {
    graphic: Graphic<D>,
    size: (usize, usize),
    pixel_size: (usize, usize),
    alt_screen_mode: bool,
    flush_cache: VecDeque<Vec<Cell>>,
    start_row: usize,
    alt_start_row: usize,
    history_size: usize,
    buffer: VecDeque<Vec<Cell>>,
    alt_buffer: VecDeque<Vec<Cell>>,
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
            start_row: 0,
            alt_start_row: 0,
            history_size: DEFAULT_HISTORY_SIZE,
        }
    }

    pub fn swap_alt_screen(&mut self, cell: Cell) {
        self.alt_screen_mode = !self.alt_screen_mode;
        swap(&mut self.buffer, &mut self.alt_buffer);
        swap(&mut self.start_row, &mut self.alt_start_row);

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
        self.buffer[self.start_row + row][col]
    }

    pub fn write(&mut self, row: usize, col: usize, cell: Cell) {
        let start_row = self.buffer.len() - self.height();
        self.buffer[start_row + row][col] = cell;
    }

    pub fn clear(&mut self, cell: Cell) {
        let start = self.start_row;
        let end = self.start_row + self.height();

        self.buffer
            .range_mut(start..end)
            .for_each(|row| row.fill(cell));
    }
}

impl<D: DrawTarget> TerminalBuffer<D> {
    pub fn flush(&mut self) {
        let start = self.start_row;
        let end = self.start_row + self.height();
        let buffer = self.buffer.range_mut(start..end);

        for (i, row) in buffer.enumerate() {
            for (j, &cell) in row.iter().enumerate() {
                if cell != self.flush_cache[i][j] {
                    self.graphic.write(i, j, cell);
                    self.flush_cache[i][j] = cell;
                }
            }
        }
    }

    pub fn full_flush(&mut self) {
        let start = self.start_row;
        let end = self.start_row + self.height();
        let buffer = self.buffer.range_mut(start..end);

        for (i, row) in buffer.enumerate() {
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
    pub fn clear_history(&mut self) {
        if !self.alt_screen_mode {
            self.buffer.drain(0..self.start_row);
            self.start_row = 0;
        }
    }

    pub fn scroll_history(&mut self, count: isize) {
        self.start_row = self
            .start_row
            .saturating_add_signed(-count)
            .min(self.buffer.len() - self.height());
    }

    pub fn resize_history(&mut self, capacity: usize) {
        self.history_size = capacity;
    }

    pub fn ensure_latest(&mut self) {
        self.start_row = self.buffer.len() - self.height();
    }
}

impl<D: DrawTarget> TerminalBuffer<D> {
    pub fn scroll_region(&mut self, count: isize, cell: Cell, region: Range<usize>) {
        let (top, bottom) = (region.start, region.end);
        let start_row = self.buffer.len() - self.height();

        if count > 0 {
            for _ in 0..count.unsigned_abs() {
                if !self.alt_screen_mode && top == 0 {
                    let row = if self.history_size + self.height() == self.buffer.len() {
                        let mut row = self.buffer.pop_back().unwrap();
                        row.fill(cell);
                        row
                    } else {
                        vec![cell; self.width()]
                    };
                    self.buffer.insert(start_row, row);
                } else {
                    let mut row = self.buffer.remove(start_row + bottom).unwrap();
                    row.fill(cell);
                    self.buffer.insert(start_row + top, row);
                }
            }
        } else {
            for _ in 0..count.unsigned_abs() {
                if !self.alt_screen_mode && bottom == self.height() - 1 {
                    if self.start_row + self.height() == self.buffer.len() {
                        self.start_row += 1;
                    }
                    let row = if self.history_size + self.height() == self.buffer.len() {
                        let mut row = self.buffer.pop_front().unwrap();
                        row.fill(cell);
                        self.start_row = self.start_row.saturating_sub(1);
                        row
                    } else {
                        vec![cell; self.width()]
                    };
                    self.buffer.push_back(row);
                } else {
                    let mut row = self.buffer.remove(start_row + top).unwrap();
                    row.fill(cell);
                    self.buffer.insert(start_row + bottom, row);
                }
            }
        }
    }
}
