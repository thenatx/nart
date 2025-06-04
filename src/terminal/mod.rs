use nix::{errno::Errno, unistd};
use pty::Pty;

pub mod pty;

pub struct TerminalState {
    pub pty: Pty,
}

impl TerminalState {
    pub fn new() -> Self {
        let defualt_shell = std::env::var("SHELL").unwrap();

        Self {
            pty: Pty::new_with_shell(&defualt_shell).unwrap(),
        }
    }

    pub fn write_content(&mut self, buf: &str) {
        match unistd::write(&self.pty.master, buf.as_bytes()) {
            Ok(_) => (),
            Err(e) => log::error!("Error writting to the master: {}", e),
        }
    }

    pub fn read_content(&mut self) -> Vec<u8> {
        let mut read_buffer = [0; 65536];
        let content = match unistd::read(&self.pty.master, &mut read_buffer) {
            Ok(bytes_read) => read_buffer[..bytes_read].to_vec(),
            Err(e) => {
                match e {
                    Errno::EAGAIN | Errno::EIO => (),
                    e => log::error!("Error while reading the master: {}", e),
                }
                Vec::new()
            }
        };

        content
    }
}
