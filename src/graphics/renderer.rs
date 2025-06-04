use super::WgpuContext;
use std::sync::Arc;
use wgpu::{LoadOp, RenderPassColorAttachment, TextureUsages, TextureViewDescriptor};
use winit::{dpi::PhysicalSize, window::Window};

use super::text::TextRenderer;

pub struct Renderer {
    window: Arc<Window>,
    wgpu_context: WgpuContext<'static>,
    text_renderer: TextRenderer,
    size: PhysicalSize<u32>,
    instance_num: u32,
}

impl Renderer {
    pub fn new(window: Window) -> Self {
        let window = Arc::new(window);
        let window_size = window.inner_size();
        let wgpu_context = WgpuContext::new(&window, window_size.width, window_size.height);
        let metrics = cosmic_text::Metrics::new(16.0, 12.0);
        let mut text_renderer = TextRenderer::new_with_metrics(
            &wgpu_context.device,
            &wgpu_context.queue,
            &wgpu_context.surf_config,
            metrics,
        );

        text_renderer.resize(
            window_size.width,
            window_size.height,
            &wgpu_context.device,
            &wgpu_context.queue,
        );

        Self {
            window,
            wgpu_context,
            text_renderer,
            size: window_size,
            instance_num: 0,
        }
    }

    pub fn init_draw(&mut self) {
        let background_color = super::Color::new(0, 0, 0, 255);
        let mut command_encoder =
            self.wgpu_context
                .device
                .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                    label: Some("Main command encoder"),
                });

        let surface_texture = self.wgpu_context.surface.get_current_texture().unwrap();
        let surface_view = surface_texture.texture.create_view(&TextureViewDescriptor {
            label: Some("Window surface texture view"),
            dimension: Some(wgpu::TextureViewDimension::D2),
            format: Some(self.wgpu_context.surf_config.format),
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
        render_pass.draw(0..6, 0..self.instance_num);

        drop(render_pass);
        self.wgpu_context.queue.submit([command_encoder.finish()]);

        surface_texture.present();
        self.window.request_redraw();
    }

    pub fn write_glyphs(&mut self, text: &str) {
        self.text_renderer.set_text(
            &self.wgpu_context.device,
            &self.wgpu_context.queue,
            text.to_string(),
        );

        self.instance_num = self.text_renderer.cache.len() as u32;
    }

    pub fn resize(&mut self, new_size: PhysicalSize<u32>) {
        if new_size.width == 0 || new_size.height == 0 {
            return;
        }

        self.size = new_size;
        self.wgpu_context
            .surface_resize(new_size.width, new_size.height);

        self.text_renderer.resize(
            self.size.width,
            self.size.height,
            &self.wgpu_context.device,
            &self.wgpu_context.queue,
        );

        self.init_draw();
    }

    pub fn window(&self) -> Arc<Window> {
        self.window.clone()
    }
}
