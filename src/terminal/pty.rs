use nix::{
    fcntl::{FcntlArg, OFlag},
    pty::ForkptyResult,
    unistd,
};
use std::{
    io,
    os::fd::{AsFd, FromRawFd, IntoRawFd},
};

#[derive(Debug)]
pub struct Pty {
    pub master: std::fs::File,
    pub child_pid: Option<i32>,
}

impl Pty {
    pub fn new_with_shell(command: &str) -> Result<Self, io::Error> {
        let forked_pty = unsafe { nix::pty::forkpty(None, None)? };

        let pty = match forked_pty {
            ForkptyResult::Parent { child, master } => {
                let master = unsafe { std::fs::File::from_raw_fd(master.into_raw_fd()) };

                let flags = nix::fcntl::fcntl(master.as_fd(), FcntlArg::F_GETFL)?;
                nix::fcntl::fcntl(
                    master.as_fd(),
                    FcntlArg::F_SETFL(OFlag::from_bits_retain(flags) | OFlag::O_NONBLOCK),
                )?;

                Self {
                    master,
                    child_pid: Some(child.as_raw()),
                }
            }
            ForkptyResult::Child => {
                let exit_code = std::process::Command::new(command).spawn()?.wait()?.code();
                std::process::exit(exit_code.unwrap_or(0));
            }
        };

        Ok(pty)
    }

    pub fn close(&mut self) {
        if let Some(pid) = self.child_pid {
            let _ = nix::sys::signal::kill(unistd::Pid::from_raw(pid), nix::sys::signal::SIGTERM);
        }
    }
}
