use alloc::collections::vec_deque::VecDeque;
use alloc::vec::Vec;
use core::mem::swap;
use core::ops::Range;

use crate::cell::Cell;
use crate::graphic::{DrawTarget, Graphic};

const INIT_SIZE: Size = (1, 1);
const DEFAULT_HISTORY_SIZE: usize = 200;

type Size = (usize, usize);

#[derive(Clone)]
pub struct Row {
    cells: Vec<Cell>,
    wrapped: bool,
}

impl Row {
    pub fn new(width: usize, cell: Cell) -> Self {
        Self {
            cells: vec![cell; width],
            wrapped: false,
        }
    }

    pub fn is_blank(&self) -> bool {
        !self.wrapped && self.cells.iter().all(|c| *c == Cell::default())
    }
}

pub struct TerminalBuffer {
    size: Size,
    pixel_size: Size,
    alt_screen_mode: bool,
    flush_cache: Vec<Vec<Cell>>,
    start_row: usize,
    alt_start_row: usize,
    history_size: usize,
    buffer: VecDeque<Row>,
    alt_buffer: VecDeque<Row>,
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
        let (cols, rows) = INIT_SIZE;
        let init_rows = vec![Row::new(cols, Cell::default()); rows];

        Self {
            size: INIT_SIZE,
            pixel_size: (0, 0),
            alt_screen_mode: false,
            flush_cache: vec![vec![Cell::default(); cols]; rows],
            start_row: 0,
            alt_start_row: 0,
            history_size: DEFAULT_HISTORY_SIZE,
            buffer: init_rows.clone().into(),
            alt_buffer: init_rows.into(),
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

    pub fn set_wrap(&mut self, row: usize, wrapped: bool) {
        let start_row = self.buffer.len() - self.height();
        if let Some(r) = self.buffer.get_mut(start_row + row) {
            r.wrapped = wrapped;
        }
    }
}

struct ReflowContext<'a> {
    new_rows: VecDeque<Row>,
    current_line: Vec<Cell>,
    pending: Vec<(usize, usize)>,
    origins: Vec<(usize, usize)>,
    results: &'a mut [(usize, usize)],
    new_width: usize,
}

impl<'a> ReflowContext<'a> {
    fn new(new_width: usize, cursors: &'a mut [(usize, usize)]) -> Self {
        Self {
            new_rows: VecDeque::new(),
            current_line: Vec::new(),
            pending: Vec::new(),
            origins: cursors.to_vec(),
            results: cursors,
            new_width,
        }
    }

    fn collect_row(&mut self, row_idx: usize, mut row: Row) {
        for (index, cursor) in self.origins.iter().enumerate() {
            let (cursor_row, cursor_col) = *cursor;
            if cursor_row == row_idx {
                let logical_offset = self.current_line.len() + cursor_col;
                self.pending.push((index, logical_offset));
            }
        }

        if !row.wrapped {
            let trim_pos = row.cells.iter().rposition(|c| *c != Cell::default());
            row.cells.truncate(trim_pos.map_or(0, |p| p + 1));
        }

        self.current_line.extend(row.cells);

        if !row.wrapped {
            self.flush();
        }
    }

    fn flush(&mut self) {
        let width = self.new_width;

        if self.current_line.is_empty() {
            self.new_rows.push_back(Row::new(width, Cell::default()));
            let out_row = self.new_rows.len() - 1;
            for (index, _) in self.pending.drain(..) {
                self.results[index] = (out_row, 0);
            }
            return;
        }

        let total_len = self.current_line.len();

        for start in (0..total_len).step_by(width) {
            let end = (start + width).min(total_len);
            let mut cells = self.current_line[start..end].to_vec();
            let wrapped = end < total_len;
            cells.resize(width, Cell::default());
            self.new_rows.push_back(Row { cells, wrapped });
        }

        let chunk_count = total_len.div_ceil(width);
        let base = self.new_rows.len() - chunk_count;
        let last_row = self.new_rows.len() - 1;

        for (index, char_offset) in self.pending.drain(..) {
            let out_row = (base + char_offset / width).min(last_row);
            let col = (char_offset % width).min(width - 1);
            self.results[index] = (out_row, col);
        }

        self.current_line.clear();
    }
}

impl TerminalBuffer {
    pub fn update_size(
        &mut self,
        font_size: Size,
        graphic_size: Size,
        cursors: &mut [(usize, usize)],
    ) {
        let width = (graphic_size.0 / font_size.0).max(1);
        let height = (graphic_size.1 / font_size.1).max(1);
        self.pixel_size = (font_size.0 * width, font_size.1 * height);

        if self.size == (width, height) {
            return;
        }

        if self.alt_screen_mode {
            swap(&mut self.buffer, &mut self.alt_buffer);
            swap(&mut self.start_row, &mut self.alt_start_row);
        }

        let old_height = self.size.1;
        let buffer_len = self.buffer.len();

        let active_cursors = if self.alt_screen_mode {
            &mut []
        } else {
            &mut *cursors
        };

        for cursor in active_cursors.iter_mut() {
            cursor.0 += buffer_len - old_height;
        }

        let cursor_count = active_cursors.len();
        let mut tracked = active_cursors
            .iter()
            .copied()
            .collect::<Vec<(usize, usize)>>();
        tracked.push((self.start_row, 0));

        let mut reflow_ctx = ReflowContext::new(width, &mut tracked);
        for (row_idx, row) in self.buffer.drain(..).enumerate() {
            reflow_ctx.collect_row(row_idx, row);
        }
        if !reflow_ctx.current_line.is_empty() {
            reflow_ctx.flush();
        }
        let mut new_buffer = reflow_ctx.new_rows;

        while new_buffer.len() > height {
            match new_buffer.back() {
                Some(row) if row.is_blank() => {
                    new_buffer.pop_back();
                }
                _ => break,
            }
        }

        if new_buffer.len() < height {
            let template = Row::new(width, Cell::default());
            new_buffer.resize(height, template);
        }

        let max_start = new_buffer.len().saturating_sub(height);
        self.start_row = tracked[cursor_count].0.min(max_start);

        let viewport_start = new_buffer.len().saturating_sub(height);
        for (index, cursor) in active_cursors.iter_mut().enumerate() {
            cursor.0 = tracked[index].0.saturating_sub(viewport_start);
            cursor.0 = cursor.0.min(height - 1);
            cursor.1 = tracked[index].1.min(width - 1);
        }

        self.buffer = new_buffer;

        for row in self.alt_buffer.iter_mut() {
            row.cells.resize(width, Cell::default());
            row.wrapped = false;
        }

        let template = Row::new(width, Cell::default());
        self.alt_buffer.resize(height, template);

        if self.alt_screen_mode {
            swap(&mut self.buffer, &mut self.alt_buffer);
            swap(&mut self.start_row, &mut self.alt_start_row);
            for cursor in cursors.iter_mut() {
                cursor.0 = cursor.0.min(height - 1);
                cursor.1 = cursor.1.min(width - 1);
            }
        }

        self.size = (width, height);
        self.flush_cache = vec![vec![Cell::default(); width]; height];
    }
}

impl TerminalBuffer {
    #[inline(always)]
    pub fn row_mut(&mut self, row: usize) -> &mut [Cell] {
        let start_row = self.buffer.len() - self.height();
        &mut self.buffer[start_row + row].cells
    }

    pub fn clear(&mut self, cell: Cell) {
        let start = self.start_row;
        let end = self.start_row + self.height();

        for row in self.buffer.range_mut(start..end) {
            row.cells.fill(cell);
            row.wrapped = false;
        }
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
            for (j, &cell) in row.cells.iter().enumerate() {
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
            for (j, &cell) in row.cells.iter().enumerate() {
                graphic.write(i, j, cell);
            }
        }

        let background = Cell::default().background;
        let rgb = graphic.color_to_rgb(background);

        for y in self.pixel_size.1..graphic.size().1 {
            for x in 0..self.pixel_size.0 {
                graphic.draw_pixel(x, y, rgb);
            }
        }
        for y in 0..graphic.size().1 {
            for x in self.pixel_size.0..graphic.size().0 {
                graphic.draw_pixel(x, y, rgb);
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
                        row.cells.fill(cell);
                        row.wrapped = false;
                        row
                    } else {
                        Row::new(self.width(), cell)
                    };
                    self.buffer.insert(start_row, row);
                } else {
                    let mut row = self.buffer.remove(start_row + bottom).unwrap();
                    row.cells.fill(cell);
                    row.wrapped = false;
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
                        row.cells.fill(cell);
                        row.wrapped = false;
                        self.start_row = self.start_row.saturating_sub(1);
                        row
                    } else {
                        Row::new(self.width(), cell)
                    };
                    self.buffer.push_back(row);
                } else {
                    let mut row = self.buffer.remove(start_row + top).unwrap();
                    row.cells.fill(cell);
                    row.wrapped = false;
                    self.buffer.insert(start_row + bottom, row);
                }
            }
        }
    }
}
