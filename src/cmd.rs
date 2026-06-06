#[derive(Debug, Default, Copy, Clone, PartialEq, Eq)]
pub struct Point {
    pub x: u16,
    pub y: u16,
}

#[derive(Debug, Default, Copy, Clone, PartialEq, Eq)]
pub struct RgbColor {
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

#[derive(Debug)]
pub struct CellUpdate {
    pub pos: Point,
    pub glyph: char,
    pub fg_color: Option<RgbColor>,
    pub bg_color: Option<RgbColor>,
}

impl Default for CellUpdate {
    fn default() -> Self {
        Self {
            pos: Point { x: 0, y: 0 },
            glyph: ' ',
            fg_color: None,
            bg_color: None,
        }
    }
}

#[derive(Debug)]
pub enum Cmd {
    Draw(CellUpdate),
    Clear,
    Flush,
}
