use crate::cmd::{CellUpdate, Cmd, Point, RgbColor};
use crossterm::{
    cursor::{Hide, MoveTo, Show},
    execute,
    style::{Color, Print, ResetColor, SetBackgroundColor, SetForegroundColor},
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use std::io::{Write, stdout};

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
struct Cell {
    glyph: char,
    fg_color: Option<RgbColor>,
    bg_color: Option<RgbColor>,
}

impl Default for Cell {
    fn default() -> Self {
        Self {
            glyph: ' ',
            fg_color: None,
            bg_color: None,
        }
    }
}

#[derive(Debug)]
struct Canvas {
    height: u16,
    width: u16,
    buffer: Vec<Cell>,
}

impl Canvas {
    fn new(width: u16, height: u16) -> Self {
        let size = (width as usize) * (height as usize);
        Self {
            width,
            height,
            buffer: vec![Cell::default(); size],
        }
    }

    fn clear(&mut self) {
        for cell in self.buffer.iter_mut() {
            *cell = Cell::default();
        }
    }
}

#[derive(Debug)]
pub struct DisplayServer {
    current_canvas: Canvas,
    next_canvas: Canvas,
    is_first_frame: bool,
}

impl DisplayServer {
    pub fn new() -> Self {
        enable_raw_mode().expect("Failed to enable raw mode");
        execute!(stdout(), EnterAlternateScreen, Hide).expect("Failed to setup terminal");

        let (width, height) = crossterm::terminal::size().unwrap_or((80, 24));

        Self {
            current_canvas: Canvas::new(width, height),
            next_canvas: Canvas::new(width, height),
            is_first_frame: true,
        }
    }

    fn draw(&mut self, cell_update: CellUpdate) {
        let Point { x, y } = cell_update.pos;

        if x >= self.next_canvas.width || y >= self.next_canvas.height {
            return;
        }

        let idx = (y * self.next_canvas.width + x) as usize;

        self.next_canvas.buffer[idx] = Cell {
            glyph: cell_update.glyph,
            fg_color: cell_update.fg_color,
            bg_color: cell_update.bg_color,
        };
    }

    fn clear(&mut self) {
        self.next_canvas.clear();
    }

    fn flush(&mut self) {
        let mut stdout = std::io::stdout();
        let width = self.current_canvas.width;
        let height = self.current_canvas.height;

        let mut last_fg_color: Option<RgbColor> = None;
        let mut last_bg_color: Option<RgbColor> = None;

        let mut cursor_pos: Option<(u16, u16)> = None;

        for y in 0..height {
            for x in 0..width {
                let idx = (y * width + x) as usize;
                let current_cell = self.current_canvas.buffer[idx];
                let next_cell = self.next_canvas.buffer[idx];

                if !self.is_first_frame && current_cell == next_cell {
                    cursor_pos = None;
                    continue;
                }

                if cursor_pos != Some((x, y)) {
                    crossterm::queue!(stdout, MoveTo(x, y)).unwrap();
                }

                match next_cell.fg_color {
                    Some(fg) if last_fg_color != next_cell.fg_color => {
                        crossterm::queue!(
                            stdout,
                            SetForegroundColor(Color::Rgb {
                                r: fg.r,
                                g: fg.g,
                                b: fg.b,
                            })
                        )
                        .unwrap();
                        last_fg_color = next_cell.fg_color;
                    }
                    None => {
                        crossterm::queue!(stdout, SetForegroundColor(Color::Reset)).unwrap();
                        last_fg_color = None;
                    }
                    _ => {}
                }

                match next_cell.bg_color {
                    Some(bg) if last_bg_color != next_cell.bg_color => {
                        crossterm::queue!(
                            stdout,
                            SetBackgroundColor(Color::Rgb {
                                r: bg.r,
                                g: bg.g,
                                b: bg.b,
                            })
                        )
                        .unwrap();
                        last_bg_color = next_cell.bg_color;
                    }
                    None => {
                        crossterm::queue!(stdout, SetBackgroundColor(Color::Reset)).unwrap();
                        last_bg_color = None;
                    }
                    _ => {}
                }

                crossterm::queue!(stdout, Print(next_cell.glyph)).unwrap();

                cursor_pos = Some((x + 1, y));
            }
        }

        self.is_first_frame = false;

        Write::flush(&mut stdout).unwrap();

        self.current_canvas
            .buffer
            .copy_from_slice(&self.next_canvas.buffer);
    }

    pub fn get_size(&mut self) -> (u16, u16) {
        (self.current_canvas.width, self.current_canvas.height)
    }

    pub fn resize(&mut self, new_width: u16, new_height: u16) {
        let new_size = (new_width * new_height) as usize;

        let mut new_current_buffer = vec![Cell::default(); new_size];
        let mut new_next_buffer = vec![Cell::default(); new_size];

        let old_width = self.current_canvas.width;
        let old_height = self.current_canvas.height;

        let min_width = std::cmp::min(old_width, new_width) as usize;
        let min_height = std::cmp::min(old_height, new_height) as usize;

        for y in 0..min_height {
            for x in 0..min_width {
                let old_idx = y * (old_width as usize) + x;
                let new_idx = y * (new_width as usize) + x;

                new_current_buffer[new_idx] = self.current_canvas.buffer[old_idx];
                new_next_buffer[new_idx] = self.next_canvas.buffer[old_idx];
            }
        }

        self.current_canvas.width = new_width;
        self.current_canvas.height = new_height;
        self.current_canvas.buffer = new_current_buffer;

        self.next_canvas.width = new_width;
        self.next_canvas.height = new_height;
        self.next_canvas.buffer = new_next_buffer;

        self.is_first_frame = true;
    }

    pub fn execute(&mut self, cmd: Cmd) {
        match cmd {
            Cmd::Draw(cell_update) => self.draw(cell_update),
            Cmd::Clear => self.clear(),
            Cmd::Flush => self.flush(),
            Cmd::QuerySize => (),
        }
    }
}

impl Drop for DisplayServer {
    fn drop(&mut self) {
        let _ = disable_raw_mode();
        let _ = crossterm::execute!(std::io::stdout(), Show, LeaveAlternateScreen, ResetColor,);
    }
}
