use super::cell::Cell;
use super::graphic::{DrawTarget, TextOnGraphic};
use alloc::vec;
use alloc::vec::Vec;

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

    pub fn width(&self) -> usize {
        self.inner.width()
    }

    pub fn height(&self) -> usize {
        self.inner.height()
    }

    pub fn clear(&mut self, cell: Cell) {
        self.inner.clear(cell);
    }

    pub fn read(&self, row: usize, col: usize) -> Cell {
        let row = row % self.inner.height();
        self.buffer[row][col]
    }

    pub fn write(&mut self, row: usize, col: usize, cell: Cell) {
        let row = row % self.inner.height();
        self.buffer[row][col] = cell;
        self.inner.write(row, col, cell);
    }

    pub fn new_line(&mut self, cell: Cell) {
        for i in 1..self.height() {
            for j in 0..self.width() {
                let last = self.read(i - 1, j);
                let current = self.read(i, j);
                if last != current {
                    self.write(i - 1, j, current);
                }
                self.write(i - 1, j, current);

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
