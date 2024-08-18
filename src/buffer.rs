use alloc::collections::vec_deque::VecDeque;
use alloc::vec::Vec;

use super::cell::Cell;
use super::config::CONFIG;
use super::graphic::{DrawTarget, TextOnGraphic};

const DEFAULT_SIZE: (usize, usize) = (1, 1);

pub struct TerminalBuffer<D: DrawTarget> {
    graphic: TextOnGraphic<D>,
    size: (usize, usize),
    flush_cache: VecDeque<Vec<Cell>>,
    buffer: VecDeque<Vec<Cell>>,
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
    pub fn new(graphic: TextOnGraphic<D>) -> Self {
        let buffer = VecDeque::from(vec![vec![Cell::default(); DEFAULT_SIZE.0]; DEFAULT_SIZE.1]);

        TerminalBuffer {
            graphic,
            size: DEFAULT_SIZE,
            buffer: buffer.clone(),
            flush_cache: buffer,
        }
    }

    pub fn update_size(&mut self, font_width: usize, font_height: usize) {
        let (old_width, old_height) = self.size;

        let width = self.graphic.width() / font_width;
        let height = self.graphic.height() / font_height;

        if width == old_width && height == old_height {
            return
        }

        let buffer = VecDeque::from(vec![vec![Cell::default(); width]; height]);

        self.size = (width, height);
        self.buffer = buffer.clone();
        self.flush_cache = buffer;
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
        if cell == self.read(row, col) {
            return
        }

        let row = row % self.height();
        self.buffer[row][col] = cell;

        if CONFIG.lock().auto_flush {
            self.graphic.write(row, col, cell);
            self.flush_cache[row][col] = cell;
        }
    }

    #[inline]
    pub fn clear(&mut self, cell: Cell) {
        self.buffer
            .iter_mut()
            .for_each(|row| row.iter_mut().for_each(|c| *c = cell));

        if CONFIG.lock().auto_flush {
            self.flush();
        }
    }

    #[inline]
    pub fn flush(&mut self) {
        for (i, row) in self.buffer.iter().enumerate() {
            for (j, &cell) in row.iter().enumerate() {
                let backend = self.flush_cache[i][j];
                if cell != backend {
                    self.graphic.write(i, j, cell);
                    self.flush_cache[i][j] = cell;
                }
            }
        }
    }

    pub fn new_line(&mut self, cell: Cell) {
        self.buffer.pop_front();
        self.buffer.push_back(vec![cell; self.width()]);

        if CONFIG.lock().auto_flush {
            self.flush();
        }
    }
}
