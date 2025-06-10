use log::error;
use winit::{
    application::ApplicationHandler,
    event::{ElementState, KeyEvent, WindowEvent},
};

use crate::{graphics::renderer::Renderer, terminal};

pub struct Nart {
    renderer: Option<Renderer>,
    terminal: terminal::TerminalState,
    content: Vec<u8>,
}

impl Nart {
    pub fn new() -> Self {
        let state = terminal::TerminalState::new();

        Self {
            renderer: None,
            terminal: state,
            content: Vec::new(),
        }
    }
}

impl ApplicationHandler for Nart {
    fn resumed(&mut self, event_loop: &winit::event_loop::ActiveEventLoop) {
        let window_attrs = winit::window::Window::default_attributes().with_title("Nart");

        let window = event_loop.create_window(window_attrs).unwrap();
        let renderer = Renderer::new(window);
        self.renderer = Some(renderer);
    }

    fn window_event(
        &mut self,
        event_loop: &winit::event_loop::ActiveEventLoop,
        _window_id: winit::window::WindowId,
        event: winit::event::WindowEvent,
    ) {
        let renderer = self.renderer.as_mut().unwrap();
        let _window = renderer.window();

        match event {
            WindowEvent::CloseRequested => {
                self.terminal.pty.close();
                event_loop.exit();
            }
            WindowEvent::RedrawRequested => {
                let mut content = self.terminal.read_content();

                if !content.is_empty() {
                    self.content.append(&mut content);
                    let text = String::from_utf8_lossy(self.content.as_slice());
                    renderer.write_glyphs(&text);
                }

                renderer.init_draw();
            }
            WindowEvent::Resized(size) => renderer.resize(size),
            WindowEvent::KeyboardInput {
                event:
                    KeyEvent {
                        text: Some(text),
                        state: ElementState::Pressed,
                        ..
                    },
                ..
            } => {
                self.terminal.write_content(text.as_str());
            }
            _ => {}
        }
    }
}

pub fn init_window() {
    let event_loop = winit::event_loop::EventLoop::new().unwrap();
    let mut app = Nart::new();

    if let Err(e) = event_loop.run_app(&mut app) {
        error!("Failed to run event loop: {e}");
    };
}
