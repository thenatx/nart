use std::{collections::HashMap, u8};

use log::info;
use vte::Parser;

#[derive(Debug, Default)]
pub struct TerminalGrid {
    rows: u32,
    columns: u32,
    cells: Vec<Vec<TerminalCell>>,
    cursor: TerminalCursor,
    width: u32,
    height: u32,
    pub cell_size: (f32, f32),
    current_style: TerminalStyle,
}

impl TerminalGrid {
    pub fn get_content(&self) -> &Vec<Vec<TerminalCell>> {
        &self.cells
    }

    pub fn get_cursor(&self) -> (f32, f32) {
        self.cursor
            .get_pixel_coords(self.cell_size.0, self.cell_size.1)
    }

    pub fn update(&mut self, data: &[u8]) {
        let mut parser = Parser::new();
        let _ = parser.advance_until_terminated(self, data);
    }

    pub fn resize(&mut self, width: u32, height: u32) {
        self.width = width;
        self.height = height;
        let (columns, rows) = (
            self.width as f32 / self.cell_size.0,
            self.height as f32 / self.cell_size.1,
        );

        self.rows = rows as u32;
        self.columns = columns as u32;
    }
}

impl vte::Perform for TerminalGrid {
    fn print(&mut self, c: char) {
        if let Some(row) = self.cells.last_mut() {
            row.push(TerminalCell {
                content: c,
                style: self.current_style,
            });
            self.cursor.move_right(1);

            if self.cursor.0 >= self.columns {
                self.cursor.move_to(0, self.cursor.1 + 1);
            }

            return;
        }

        let mut row = Vec::with_capacity(self.columns as usize);
        row.push(TerminalCell {
            content: c,
            style: self.current_style,
        });
        self.cursor.move_to(1, 0);
        self.cells.push(row);
    }

    fn csi_dispatch(
        &mut self,
        params: &vte::Params,
        intermediates: &[u8],
        _ignore: bool,
        action: char,
    ) {
        let params = params.iter().flatten().copied().collect::<Vec<_>>();

        // TODO: Refactor this, looks really bad and still are ansi codes without being handled properly
        match action {
            'A' | 'B' | 'C' | 'D' => {
                let distance = *params.get(0).unwrap_or(&0) as u32;
                match action {
                    'A' => self.cursor.move_up(distance),
                    'B' => self.cursor.move_down(distance),
                    'C' => self.cursor.move_right(distance),
                    'D' => self.cursor.move_left(distance),
                    _ => (),
                }
            }
            'E' => {
                let value = *params.get(0).unwrap_or(&1) as u32;
                self.cursor.move_to(0, self.cursor.0 + value)
            }
            'F' => {
                let value = *params.get(0).unwrap_or(&1) as u32;
                self.cursor.move_to(0, self.cursor.0 - value)
            }
            'G' => {
                let value = *params.get(0).unwrap_or(&0) as u32;
                self.cursor.move_to(value, self.cursor.0)
            }

            'H' | 'f' => {
                self.cursor.move_to(
                    *params.get(0).unwrap_or(&0) as u32,
                    *params.get(1).unwrap_or(&0) as u32,
                );
            }
            'J' => {
                let value = params.get(0).unwrap_or(&0);
                // TODO: implemnt cases for 0,1 and 2 when implement an actual scrollback
                match value {
                    3 => self.cells.clear(),
                    _ => (),
                }
            }
            'K' => {
                let value = params.get(0).unwrap_or(&0);
                if let Some(line) = self.cells.get_mut(self.cursor.1 as usize) {
                    match value {
                        0 => {
                            let mut updated_line =
                                line.get(0..self.cursor.0 as usize).unwrap_or(&[]).to_vec();
                            line.clear();
                            line.append(&mut updated_line);
                        }
                        1 => {
                            let mut updated_line = line
                                .get(self.cursor.0 as usize..line.len())
                                .unwrap_or(&[])
                                .to_vec();

                            line.clear();
                            line.append(&mut updated_line);
                        }
                        2 => line.clear(),
                        _ => (),
                    }
                };
            }

            'm' => {
                let eight_bit_color_table = {
                    let mut table = HashMap::new();
                    fill_color_table(&mut table);

                    table
                };

                let mut i = 0;
                while i < params.len() {
                    let param = params[i];
                    self.current_style.foreground = match param {
                        0 => TerminalColor::White,
                        30 => TerminalColor::Black,
                        31 => TerminalColor::Red,
                        32 => TerminalColor::Green,
                        33 => TerminalColor::Yellow,
                        34 => TerminalColor::Blue,
                        35 => TerminalColor::Magenta,
                        36 => TerminalColor::Cyan,
                        37 => TerminalColor::White,
                        38 => {
                            if i + 1 >= params.len() {
                                i += 2;
                                continue;
                            }

                            let color = if params[i + 1] == 2 {
                                let r = *params.get(i + 2).unwrap_or(&0) as u8;
                                let g = *params.get(i + 3).unwrap_or(&0) as u8;
                                let b = *params.get(i + 4).unwrap_or(&0) as u8;

                                TerminalColor::Rgb(r, g, b)
                            } else if params[i + 1] == 5 {
                                let color_index = *params.get(i + 2).unwrap_or(&0) as u8;
                                match color_index {
                                    // TODO: found a better way to handle this case to avoid repetition
                                    code @ 0..16 => match code {
                                        0 => TerminalColor::Black,
                                        1 => TerminalColor::Red,
                                        2 => TerminalColor::Green,
                                        3 => TerminalColor::Yellow,
                                        4 => TerminalColor::Blue,
                                        5 => TerminalColor::Magenta,
                                        6 => TerminalColor::Cyan,
                                        7 => TerminalColor::White,
                                        8 => TerminalColor::BrightBlack,
                                        9 => TerminalColor::BrightRed,
                                        10 => TerminalColor::BrightGreen,
                                        12 => TerminalColor::BrightYellow,
                                        13 => TerminalColor::BrightBlue,
                                        14 => TerminalColor::BrightMagenta,
                                        15 => TerminalColor::BrightCyan,
                                        16 => TerminalColor::BrightWhite,
                                        _ => {
                                            unreachable!()
                                        }
                                    },
                                    code @ 16..231 => eight_bit_color_table
                                        .get(&code)
                                        .cloned()
                                        .unwrap_or(TerminalColor::White),
                                    code @ 231..255 => {
                                        let gray = ((code - 231) * 10 + 8) as u8;

                                        TerminalColor::Rgb(gray, gray, gray)
                                    }
                                    u8::MAX => {
                                        unreachable!()
                                    }
                                }
                            } else {
                                i += 2;
                                continue;
                            };

                            i += 3;
                            color
                        }
                        39 => TerminalColor::White,
                        90 => TerminalColor::BrightBlack,
                        91 => TerminalColor::BrightRed,
                        92 => TerminalColor::BrightGreen,
                        93 => TerminalColor::BrightYellow,
                        94 => TerminalColor::BrightBlue,
                        95 => TerminalColor::BrightMagenta,
                        96 => TerminalColor::BrightCyan,
                        97 => TerminalColor::BrightWhite,
                        _ => {
                            i += 1;
                            continue;
                        }
                    };
                    i += 1;
                }
            }
            _ => (),
        };

        info!(
            "Params={:?}, Action={:?}, intermediates={:?}",
            params, action, intermediates
        );
    }

    fn execute(&mut self, byte: u8) {
        match byte {
            10 => {
                if let Some(row) = self.cells.last_mut() {
                    self.cursor.move_to(0, self.cursor.1 + 1);
                    row.push(TerminalCell {
                        content: '\n',
                        style: self.current_style,
                    });
                }
            }
            _ => (),
        }
    }
}

#[derive(Debug, Default, Clone, Copy)]
pub struct TerminalCell {
    pub style: TerminalStyle,
    pub content: char,
}

#[derive(Debug, Default)]
struct TerminalCursor(u32, u32);

impl TerminalCursor {
    fn move_up(&mut self, y: u32) {
        if self.1 == 0 {
            return;
        }

        self.1 = self.1 - y
    }

    fn move_down(&mut self, y: u32) {
        self.1 = self.1 + y
    }

    fn move_left(&mut self, x: u32) {
        if self.0 == 0 {
            return;
        }

        self.0 = self.0 - x
    }

    fn move_right(&mut self, x: u32) {
        self.0 = self.0 + x
    }

    fn move_to(&mut self, x: u32, y: u32) {
        self.0 = x;
        self.1 = y;
    }

    fn get_pixel_coords(&self, width: f32, height: f32) -> (f32, f32) {
        (width * self.0 as f32, height * self.1 as f32)
    }
}

#[derive(Debug, Clone, Copy)]
pub struct TerminalStyle {
    pub foreground: TerminalColor,
}

impl Default for TerminalStyle {
    fn default() -> Self {
        Self {
            foreground: TerminalColor::White,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub enum TerminalColor {
    Black,
    Red,
    Green,
    Blue,
    Yellow,
    Cyan,
    Magenta,
    White,
    BrightBlack,
    BrightRed,
    BrightGreen,
    BrightBlue,
    BrightYellow,
    BrightCyan,
    BrightMagenta,
    BrightWhite,
    Rgb(u8, u8, u8),
}

fn fill_color_table(table: &mut HashMap<u8, TerminalColor>) {
    // This is basically copied from wikipedia, seems like gives different results than other terminals
    // i should check this out later
    for red in 0..6 {
        for green in 0..6 {
            for blue in 0..6 {
                let code = 16 + (red * 36) + (green * 6) + blue;
                let r = if red == 0 { 0 } else { red * 40 + 55 };
                let g = if green == 0 { 0 } else { green * 40 + 55 };
                let b = if blue == 0 { 0 } else { blue * 40 + 55 };

                table.insert(code, TerminalColor::Rgb(r, g, b));
            }
        }
    }
}
