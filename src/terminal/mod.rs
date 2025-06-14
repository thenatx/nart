use grid::TerminalGrid;
use nix::{errno::Errno, unistd};
use pty::Pty;

pub mod grid;
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

    pub fn resize_grid(&mut self, new_size: (u32, u32), cell_size: (f32, f32)) {
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
