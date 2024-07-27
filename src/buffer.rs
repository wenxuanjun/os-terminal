use alloc::vec;
use alloc::vec::Vec;

use super::cell::Cell;
use super::graphic::{DrawTarget, TextOnGraphic};

pub struct TerminalBuffer<D: DrawTarget> {
    buffer: Vec<Vec<Cell>>,
    inner: TextOnGraphic<D>,
}

impl<D: DrawTarget> TerminalBuffer<D> {
    pub fn new(inner: TextOnGraphic<D>) -> Self {
        TerminalBuffer {
            buffer: vec![vec![Cell::default(); inner.width()]; inner.height()],
            inner,
        }
    }

    #[inline]
    pub fn width(&self) -> usize {
        self.inner.width()
    }

    #[inline]
    pub fn height(&self) -> usize {
        self.inner.height()
    }

    #[inline]
    pub fn clear(&mut self, cell: Cell) {
        self.buffer
            .iter_mut()
            .for_each(|row| row.iter_mut().for_each(|c| *c = cell));
        self.inner.clear(cell);
    }

    #[inline]
    pub fn read(&self, row: usize, col: usize) -> Cell {
        let row = row % self.inner.height();
        self.buffer[row][col]
    }

    #[inline]
    pub fn write(&mut self, row: usize, col: usize, cell: Cell) {
        let row = row % self.inner.height();
        self.buffer[row][col] = cell;
        self.inner.write(row, col, cell);
    }

    pub fn new_line(&mut self, cell: Cell) {
        let mut prev_row = (0..self.width())
            .map(|j| self.read(0, j))
            .collect::<Vec<_>>();

        for i in 1..self.height() {
            for j in 0..self.width() {
                let current = self.read(i, j);
                if prev_row[j] != current {
                    self.write(i - 1, j, current);
                }
                prev_row[j] = current;
            }
        }

        for j in 0..self.width() {
            let current = self.read(self.height() - 1, j);
            if current != cell {
                self.write(self.height() - 1, j, cell);
            }
        }
    }
}
