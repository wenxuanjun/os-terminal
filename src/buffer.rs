use alloc::collections::vec_deque::VecDeque;
use alloc::vec::Vec;
use core::mem::swap;
use core::ops::Range;

use crate::cell::Cell;
use crate::graphic::{DrawTarget, Graphic};

const INIT_SIZE: Size = (1, 1);
const DEFAULT_HISTORY_SIZE: usize = 200;

type Size = (usize, usize);

pub struct TerminalBuffer {
    size: Size,
    pixel_size: Size,
    alt_screen_mode: bool,
    flush_cache: Vec<Vec<Cell>>,
    start_row: usize,
    alt_start_row: usize,
    history_size: usize,
    buffer: VecDeque<Vec<Cell>>,
    alt_buffer: VecDeque<Vec<Cell>>,
}

impl TerminalBuffer {
    pub fn width(&self) -> usize {
        self.size.0
    }

    pub fn height(&self) -> usize {
        self.size.1
    }
}

impl Default for TerminalBuffer {
    fn default() -> Self {
        let buffer = vec![vec![Cell::default(); INIT_SIZE.0]; INIT_SIZE.1];

        Self {
            size: INIT_SIZE,
            pixel_size: (0, 0),
            alt_screen_mode: false,
            buffer: buffer.clone().into(),
            alt_buffer: buffer.clone().into(),
            flush_cache: buffer,
            start_row: 0,
            alt_start_row: 0,
            history_size: DEFAULT_HISTORY_SIZE,
        }
    }
}

impl TerminalBuffer {
    pub fn swap_alt_screen(&mut self, cell: Cell) {
        self.alt_screen_mode = !self.alt_screen_mode;
        swap(&mut self.buffer, &mut self.alt_buffer);
        swap(&mut self.start_row, &mut self.alt_start_row);

        if self.alt_screen_mode {
            self.clear(cell);
        }
    }

    pub fn update_size(&mut self, font_size: Size, graphic_size: Size) {
        let width = graphic_size.0 / font_size.0;
        let height = graphic_size.1 / font_size.1;
        self.pixel_size = (font_size.0 * width, font_size.1 * height);

        if self.size != (width, height) {
            let buffer = vec![vec![Cell::default(); width]; height].into();
            self.size = (width, height);
            self.buffer.clone_from(&buffer);
            self.alt_buffer.clone_from(&buffer);
            self.flush_cache = buffer.into();
        }
    }
}

impl TerminalBuffer {
    pub fn row_mut(&mut self, row: usize) -> &mut [Cell] {
        let start_row = self.buffer.len() - self.height();
        &mut self.buffer[start_row + row]
    }

    pub fn clear(&mut self, cell: Cell) {
        let start = self.start_row;
        let end = self.start_row + self.height();

        self.buffer
            .range_mut(start..end)
            .for_each(|row| row.fill(cell));
    }
}

impl TerminalBuffer {
    pub fn flush<D>(&mut self, graphic: &mut Graphic<D>)
    where
        D: DrawTarget,
    {
        let start = self.start_row;
        let end = self.start_row + self.height();
        let buffer = self.buffer.range_mut(start..end);

        for (i, row) in buffer.enumerate() {
            for (j, &cell) in row.iter().enumerate() {
                if cell != self.flush_cache[i][j] {
                    graphic.write(i, j, cell);
                    self.flush_cache[i][j] = cell;
                }
            }
        }
    }

    pub fn full_flush<D>(&mut self, graphic: &mut Graphic<D>)
    where
        D: DrawTarget,
    {
        let start = self.start_row;
        let end = self.start_row + self.height();
        let buffer = self.buffer.range_mut(start..end);

        for (i, row) in buffer.enumerate() {
            for (j, &cell) in row.iter().enumerate() {
                graphic.write(i, j, cell);
            }
        }

        let background = Cell::default().background;
        let rgb = graphic.color_to_rgb(background);
        let pixel = graphic.rgb_to_pixel(rgb);

        for y in self.pixel_size.1..graphic.size().1 {
            for x in 0..self.pixel_size.0 {
                graphic.draw_pixel(x, y, pixel);
            }
        }
        for y in 0..graphic.size().1 {
            for x in self.pixel_size.0..graphic.size().0 {
                graphic.draw_pixel(x, y, pixel);
            }
        }
    }
}

impl TerminalBuffer {
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

impl TerminalBuffer {
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
