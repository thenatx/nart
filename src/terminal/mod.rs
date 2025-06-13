use log::info;
use nix::{errno::Errno, unistd};
use pty::Pty;
use vte::Parser;

pub mod pty;

pub struct TerminalState {
    pub pty: Pty,
    pub grid: TerminalGrid,
}

impl TerminalState {
    pub fn new() -> Self {
        let defualt_shell = std::env::var("SHELL").unwrap();

        Self {
            pty: Pty::new_with_shell(&defualt_shell).unwrap(),
            grid: TerminalGrid::new(),
        }
    }

    pub fn write_content(&mut self, buf: &str) {
        match unistd::write(&self.pty.master, buf.as_bytes()) {
            Ok(_) => (),
            Err(e) => log::error!("Error writting to the master: {e}"),
        }
    }

    pub fn resize_grid(&mut self, new_size: (u32, u32), cell_size: (u32, u32)) {
        self.grid.cell_size = cell_size;
        self.grid.resize(new_size.0, new_size.1);
    }

    pub fn read_content(&mut self) -> Vec<u8> {
        let mut read_buffer = [0; 65536];
        let content = match unistd::read(&self.pty.master, &mut read_buffer) {
            Ok(bytes_read) => read_buffer[..bytes_read].to_vec(),
            Err(e) => {
                match e {
                    Errno::EAGAIN | Errno::EIO => (),
                    e => log::error!("Error while reading the master: {e}"),
                }
                Vec::new()
            }
        };

        content
    }
}

#[derive(Debug, Default)]
pub struct TerminalGrid {
    rows: u32,
    columns: u32,
    cells: Vec<Vec<TerminalCell>>,
    cursor: TerminalCursor,
    width: u32,
    height: u32,
    pub cell_size: (u32, u32),
}

impl TerminalGrid {
    pub fn new() -> Self {
        let cursor = TerminalCursor::default();

        let (rows, columns) = (0, 0);
        Self {
            rows,
            columns,
            cursor,
            // TODO: calculate this based on the font
            cell_size: (10, 8),
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

        info!("Terminal content: [ {} ]", content);

        content
    }

    pub fn get_cursor(&self) -> (u32, u32) {
        info!("Cursor is at: {:?}", self.cursor);
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
            self.width / self.cell_size.0,
            self.height / self.cell_size.1,
        );

        self.rows = rows;
        self.columns = columns;
    }
}

impl vte::Perform for TerminalGrid {
    fn print(&mut self, c: char) {
        if let Some(row) = self.cells.last_mut() {
            row.push(TerminalCell { content: c });
            self.cursor.cmove(self.cursor.0 + 1, self.cursor.1);
            return;
        }

        let mut row = Vec::with_capacity(self.columns as usize);
        row.push(TerminalCell { content: c });
        self.cursor.cmove(0, 0);
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

        match action {
            _ => (),
        };

        log::info!(
            "Params={:#?}  Action={:#?} intermediates={:#?}",
            params,
            action,
            intermediates
        )
    }

    fn execute(&mut self, byte: u8) {
        match byte {
            10 => {
                if let Some(row) = self.cells.last_mut() {
                    self.cursor.cmove(0, self.cursor.1 + 1);
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
    fn cmove(&mut self, x: u32, y: u32) {
        self.0 = x;
        self.1 = y;
    }

    fn get_pixel_coords(&self, width: u32, height: u32) -> (u32, u32) {
        (width * self.0, height * self.1)
    }
}
