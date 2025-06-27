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
}

impl TerminalGrid {
    pub fn new() -> Self {
        let cursor = TerminalCursor::default();
        let (rows, columns) = (0, 0);

        Self {
            rows,
            columns,
            cursor,
            cell_size: (0.0, 0.0),
            cells: Vec::new(),
            width: 0,
            height: 0,
        }
    }

    pub fn get_content(&self) -> String {
        let content = self
            .cells
            .iter()
            .flat_map(|c| c.iter().map(|c| c.content))
            .collect();

        content
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
            row.push(TerminalCell { content: c });
            self.cursor.move_right(1);

            if self.cursor.0 >= self.columns {
                self.cursor.move_to(0, self.cursor.1 + 1);
            }

            return;
        }

        let mut row = Vec::with_capacity(self.columns as usize);
        row.push(TerminalCell { content: c });
        self.cursor.move_to(0, 0);
        self.cells.push(row);
    }

    fn csi_dispatch(
        &mut self,
        params: &vte::Params,
        intermediates: &[u8],
        ignore: bool,
        action: char,
    ) {
        if ignore {
            return;
        }

        let params = params.iter().collect::<Vec<_>>();
        let values = params.get(0).map(|v| *v).unwrap_or_default();
        match action {
            'A' | 'B' | 'C' | 'D' => {
                let distance = *values.get(0).unwrap_or(&0) as u32;
                match action {
                    'A' => self.cursor.move_up(distance),
                    'B' => self.cursor.move_down(distance),
                    'C' => self.cursor.move_right(distance),
                    'D' => self.cursor.move_left(distance),
                    _ => (),
                }
            }
            'E' => {
                let value = *values.get(0).unwrap_or(&1) as u32;
                self.cursor.move_to(0, self.cursor.0 + value)
            }
            'F' => {
                let value = *values.get(0).unwrap_or(&1) as u32;
                self.cursor.move_to(0, self.cursor.0 - value)
            }
            'G' => {
                let value = *values.get(0).unwrap_or(&0) as u32;
                self.cursor.move_to(value, self.cursor.0)
            }

            'H' | 'f' => {
                self.cursor.move_to(
                    *values.get(0).unwrap_or(&0) as u32,
                    *values.get(1).unwrap_or(&0) as u32,
                );
            }
            'J' => {
                let value = values.get(0).unwrap_or(&0);
                // TODO: implemnt cases for 0,1 and 2 when the implement an actual scrollback buffer
                match value {
                    3 => self.cells.clear(),
                    _ => (),
                }
            }
            'K' => {
                let value = values.get(0).unwrap_or(&0);
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
                    row.push(TerminalCell { content: '\n' });
                }
            }
            _ => (),
        }
    }
}

#[derive(Debug, Default, Clone, Copy)]
pub struct TerminalCell {
    content: char,
}

#[derive(Debug, Default)]
pub struct TerminalCursor(u32, u32);

impl TerminalCursor {
    fn move_up(&mut self, y: u32) {
        self.1 = self.1 - y
    }

    fn move_down(&mut self, y: u32) {
        self.1 = self.1 + y
    }

    fn move_left(&mut self, x: u32) {
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
