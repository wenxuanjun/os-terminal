use alloc::collections::vec_deque::VecDeque;
use alloc::vec::Vec;

use super::cell::Cell;
use super::graphic::{DrawTarget, TextOnGraphic};

const DEFAULT_SIZE: (usize, usize) = (1, 1);
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

    pub fn resize(&mut self, new_capacity: usize) {
        self.capacity = new_capacity;
        while self.data.len() > new_capacity {
            self.data.pop_front();
        }
        self.data.shrink_to(new_capacity);
    }
}

pub struct TerminalBuffer<D: DrawTarget> {
    graphic: TextOnGraphic<D>,
    size: (usize, usize),
    flush_cache: VecDeque<Vec<Cell>>,
    buffer: VecDeque<Vec<Cell>>,
    above_buffer: FixedStack<Vec<Cell>>,
    below_buffer: FixedStack<Vec<Cell>>,
}

impl<D: DrawTarget> TerminalBuffer<D> {
    #[inline]
    pub const fn width(&self) -> usize {
        self.size.0
    }

    #[inline]
    pub const fn height(&self) -> usize {
        self.size.1
    }
}

impl<D: DrawTarget> TerminalBuffer<D> {
    pub fn new(graphic: TextOnGraphic<D>) -> Self {
        let buffer = vec![vec![Cell::default(); DEFAULT_SIZE.0]; DEFAULT_SIZE.1];

        Self {
            graphic,
            size: DEFAULT_SIZE,
            buffer: buffer.clone().into(),
            flush_cache: buffer.into(),
            above_buffer: FixedStack::new(DEFAULT_HISTORY_SIZE),
            below_buffer: FixedStack::new(DEFAULT_HISTORY_SIZE),
        }
    }

    pub fn update_size(&mut self, font_width: usize, font_height: usize) {
        let width = self.graphic.width() / font_width;
        let height = self.graphic.height() / font_height;

        if self.size != (width, height) {
            let buffer = vec![vec![Cell::default(); width]; height].into();
            self.size = (width, height);
            self.buffer.clone_from(&buffer);
            self.flush_cache = buffer;
        }
    }
}

impl<D: DrawTarget> TerminalBuffer<D> {
    #[inline]
    pub fn read(&self, row: usize, col: usize) -> Cell {
        self.buffer[row % self.height()][col]
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

    #[inline]
    pub fn new_line(&mut self, cell: Cell) {
        self.scroll_up(1, cell);
    }
}

impl<D: DrawTarget> TerminalBuffer<D> {
    #[inline]
    pub fn is_latest(&self) -> bool {
        self.below_buffer.is_empty()
    }

    #[inline]
    pub fn back_to_latest(&mut self) {
        self.scroll_history_up(self.below_buffer.len());
    }

    #[inline]
    pub fn clear_history(&mut self) {
        self.above_buffer.clear();
        self.below_buffer.clear();
    }

    #[inline]
    pub fn resize_history(&mut self, new_capacity: usize) {
        self.above_buffer.resize(new_capacity);
        self.below_buffer.resize(new_capacity);
    }
}

impl<D: DrawTarget> TerminalBuffer<D> {
    pub fn scroll_up(&mut self, count: usize, cell: Cell) {
        for _ in 0..count {
            if let Some(row) = self.buffer.pop_front() {
                self.above_buffer.push(row);
            }
            self.buffer.push_back(vec![cell; self.width()]);
        }
    }

    pub fn scroll_down(&mut self, count: usize, cell: Cell) {
        for _ in 0..count {
            if let Some(row) = self.buffer.pop_back() {
                self.below_buffer.push(row);
            }
            self.buffer.push_front(vec![cell; self.width()]);
        }
    }

    pub fn scroll_history_up(&mut self, count: usize) {
        let moves = count.min(self.below_buffer.len());
        for _ in 0..moves {
            if let Some(row) = self.buffer.pop_front() {
                self.above_buffer.push(row);
                self.buffer.push_back(self.below_buffer.pop().unwrap());
            }
        }
    }

    pub fn scroll_history_down(&mut self, count: usize) {
        let moves = count.min(self.above_buffer.len());
        for _ in 0..moves {
            if let Some(row) = self.buffer.pop_back() {
                self.below_buffer.push(row);
                self.buffer.push_front(self.above_buffer.pop().unwrap());
            }
        }
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

    #[inline]
    pub fn full_flush(&mut self) {
        self.buffer
            .iter_mut()
            .flat_map(|row| row.iter_mut())
            .for_each(|c| *c = c.reset_color());

        for (i, row) in self.buffer.iter().enumerate() {
            for (j, &cell) in row.iter().enumerate() {
                self.graphic.write(i, j, cell);
            }
        }
    }
}
