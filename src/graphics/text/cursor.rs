use bytemuck::{Pod, Zeroable};
use wgpu::{
    include_wgsl, vertex_attr_array, BlendState, ColorWrites, Device, Queue, RenderPass,
    RenderPipeline, SurfaceConfiguration, VertexAttribute, VertexStepMode,
};

use crate::graphics::{buffer::VertexBuffer, pipeline::PipelineBuilder};

pub struct CursorRenderer {
    pipeline: RenderPipeline,
    buffer: VertexBuffer<Cursor>,
    position: (f32, f32),
    size: (f32, f32),
    surface_size: (f32, f32),
}

impl CursorRenderer {
    pub fn new(device: &Device, surface_config: &SurfaceConfiguration) -> Self {
        let shader_desc = include_wgsl!("../../../shaders/cursor.wgsl");
        let shader_module = device.create_shader_module(shader_desc);
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            bind_group_layouts: &[],
            label: Some("Cursor pipeline layout"),
            push_constant_ranges: &[],
        });

        let pipeline = PipelineBuilder::new(device, "Cursor pipeline")
            .with_shader(&shader_module)
            .with_layout(&pipeline_layout)
            .add_color_target(
                surface_config.format,
                Some(BlendState::REPLACE),
                ColorWrites::ALL,
            )
            .add_vertex_layout(
                &Cursor::attributes(),
                std::mem::size_of::<Cursor>() as u64,
                VertexStepMode::Instance,
            )
            .build();

        let buffer = VertexBuffer::new(device, "Cursor buffer", Some(&[Cursor::default()]));
        let surface_size = (surface_config.width as f32, surface_config.height as f32);

        Self {
            pipeline,
            buffer,
            position: (0.0, 0.0),
            size: (0.0, 0.0),
            surface_size,
        }
    }

    pub fn update_cursor(
        &mut self,
        device: &Device,
        queue: &Queue,
        pos: (f32, f32),
        size: (f32, f32),
    ) {
        let new_cursor = Cursor::from_pixel(pos, size, self.surface_size);
        self.buffer.write(device, queue, &[new_cursor]);

        self.size = size;
        self.position = pos;
    }

    pub fn resize(&mut self, device: &Device, queue: &Queue, new_size: (u32, u32)) {
        self.surface_size = (new_size.0 as f32, new_size.1 as f32);
        let new_cursor = Cursor::from_pixel(self.position, self.size, self.surface_size);
        self.buffer.write(device, queue, &[new_cursor]);
    }

    pub fn draw(&self, render_pass: &mut RenderPass) {
        render_pass.set_pipeline(&self.pipeline);
        render_pass.set_vertex_buffer(0, self.buffer.raw_buffer().slice(..));
        render_pass.draw(0..6, 0..1);
    }
}

#[derive(Debug, Default, Clone, Copy, Pod, Zeroable)]
#[repr(C)]
pub struct Cursor {
    position: [f32; 2],
    size: [f32; 2],
}

impl Cursor {
    fn from_pixel(position: (f32, f32), size: (f32, f32), surface_size: (f32, f32)) -> Self {
        let [x, y, w, h] = [
            position.0 / surface_size.0 * 2.0 - 1.0,
            1.0 - position.1 / surface_size.1 * 2.0,
            size.0 / surface_size.0 * 2.0,
            size.1 / surface_size.1 * 2.0,
        ];

        Self {
            position: [x, y],
            size: [w, h],
        }
    }

    pub fn attributes() -> [VertexAttribute; 2] {
        vertex_attr_array![
          0 => Float32x2,
          1 => Float32x2
        ]
    }
}
