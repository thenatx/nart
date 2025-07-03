use crate::terminal::grid::{TerminalCell, TerminalColor};

use super::{
    text::{cursor::CursorRenderer, StyledCharacter},
    WgpuContext,
};
use std::sync::Arc;
use wgpu::{LoadOp, RenderPassColorAttachment, TextureUsages, TextureViewDescriptor};
use winit::{dpi::PhysicalSize, window::Window};

use super::text::TextRenderer;

const FONT_SIZE: f32 = 16.0;
const FONT_WIDTH: f32 = FONT_SIZE * 0.6;
const LINE_HEIGHT: f32 = FONT_SIZE * 1.2;

pub struct Renderer {
    window: Arc<Window>,
    context: WgpuContext<'static>,
    text_renderer: TextRenderer,
    cursor_renderer: CursorRenderer,
    size: PhysicalSize<u32>,
}

impl Renderer {
    pub fn new(window: Window) -> Self {
        let window = Arc::new(window);
        let window_size = window.inner_size();
        let context = WgpuContext::new(&window, window_size.width, window_size.height);

        let scale_factor = window.scale_factor() as f32;
        let metrics =
            cosmic_text::Metrics::new(FONT_SIZE / scale_factor, LINE_HEIGHT / scale_factor);

        let cursor_renderer = CursorRenderer::new(&context.device, &context.surf_config);
        let mut text_renderer = TextRenderer::new_with_metrics(
            &context.device,
            &context.queue,
            &context.surf_config,
            metrics,
        );

        text_renderer.resize(
            window_size.width,
            window_size.height,
            &context.device,
            &context.queue,
        );

        Self {
            window,
            context,
            text_renderer,
            cursor_renderer,
            size: window_size,
        }
    }

    pub fn init_draw(&mut self) {
        let background_color = super::Color::new(0, 0, 0, 255);
        let mut command_encoder =
            self.context
                .device
                .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                    label: Some("Main command encoder"),
                });

        let surface_texture = self.context.surface.get_current_texture().unwrap();
        let surface_view = surface_texture.texture.create_view(&TextureViewDescriptor {
            label: Some("Window surface texture view"),
            dimension: Some(wgpu::TextureViewDimension::D2),
            format: Some(self.context.surf_config.format),
            usage: Some(TextureUsages::RENDER_ATTACHMENT),
            ..Default::default()
        });

        let mut render_pass = command_encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("Main render pass"),
            color_attachments: &[Some(RenderPassColorAttachment {
                ops: wgpu::Operations {
                    load: LoadOp::Clear(background_color.into()),
                    store: wgpu::StoreOp::Store,
                },
                resolve_target: None,
                view: &surface_view,
            })],
            ..Default::default()
        });

        self.text_renderer.draw(&mut render_pass);
        self.cursor_renderer.draw(&mut render_pass);

        drop(render_pass);
        self.context.queue.submit([command_encoder.finish()]);

        surface_texture.present();
        self.window.request_redraw();
    }

    pub fn write_content(&mut self, content: &Vec<Vec<TerminalCell>>) {
        let content = content
            .iter()
            .flatten()
            .map(|i| {
                let color = match i.style.foreground {
                    TerminalColor::Black => super::Color::new(0, 0, 0, 255),
                    TerminalColor::Red => super::Color::new(255, 0, 0, 255),
                    TerminalColor::Green => super::Color::new(0, 255, 0, 255),
                    TerminalColor::Yellow => super::Color::new(255, 255, 0, 255),
                    TerminalColor::Blue => super::Color::new(0, 0, 255, 255),
                    TerminalColor::Magenta => super::Color::new(255, 0, 255, 255),
                    TerminalColor::Cyan => super::Color::new(0, 255, 255, 255),
                    TerminalColor::White => super::Color::new(255, 255, 255, 255),
                    TerminalColor::BrightBlack => super::Color::new(100, 100, 100, 255),
                    TerminalColor::BrightRed => super::Color::new(255, 100, 100, 255),
                    TerminalColor::BrightGreen => super::Color::new(100, 255, 100, 255),
                    TerminalColor::BrightYellow => super::Color::new(255, 255, 100, 255),
                    TerminalColor::BrightBlue => super::Color::new(100, 100, 255, 255),
                    TerminalColor::BrightMagenta => super::Color::new(255, 100, 255, 255),
                    TerminalColor::BrightCyan => super::Color::new(100, 255, 255, 255),
                    TerminalColor::BrightWhite => super::Color::new(255, 255, 255, 255),
                    TerminalColor::Rgb(r, g, b) => super::Color::new(r, g, b, 255),
                };

                StyledCharacter::new(i.content.to_string(), color)
            })
            .collect::<Vec<StyledCharacter>>();
        self.text_renderer.add_text(
            &self.context.device,
            &self.context.queue,
            content.as_slice(),
        );
    }

    pub fn get_cell_size(&mut self) -> (f32, f32) {
        if let Some(size) = self.text_renderer.get_glyph_size() {
            return size;
        }

        return (FONT_WIDTH, LINE_HEIGHT);
    }

    pub fn update_cursor(&mut self, x: f32, y: f32, size: (f32, f32)) {
        let (width, height) = size;
        self.cursor_renderer.update_cursor(
            &self.context.device,
            &self.context.queue,
            (x, y),
            (width, height),
        );
    }

    pub fn resize(&mut self, new_size: PhysicalSize<u32>) {
        if new_size.width == 0 || new_size.height == 0 {
            return;
        }

        self.size = new_size;
        self.context.surface_resize(new_size.width, new_size.height);

        self.text_renderer.resize(
            self.size.width,
            self.size.height,
            &self.context.device,
            &self.context.queue,
        );

        self.cursor_renderer.resize(
            &self.context.device,
            &self.context.queue,
            (self.size.width, self.size.height),
        );

        self.init_draw();
    }

    pub fn window(&self) -> Arc<Window> {
        self.window.clone()
    }
}
