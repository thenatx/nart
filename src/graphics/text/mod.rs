pub mod atlas;

use std::collections::HashMap;

use bytemuck::{Pod, Zeroable};
use cosmic_text::{FontSystem, SwashCache};
use wgpu::{
    include_wgsl, BindGroup, BindGroupLayout, BlendState, ColorWrites, Device, Queue, RenderPass,
    RenderPipeline, SurfaceConfiguration, VertexAttribute,
};

use atlas::{Glyph, GlyphAtlas, GlyphRectId};

use super::{buffer::VertexBuffer, pipeline::PipelineBuilder};

pub struct TextRenderer {
    pub atlas: GlyphAtlas,
    pub glyph_buffer: VertexBuffer<GlyphToRender>,
    pub cache: Vec<GlyphToRender>,
    placement_cache: HashMap<u16, cosmic_text::Placement>,
    font_system: FontSystem,
    swash_cache: SwashCache,
    attributes: cosmic_text::Attrs<'static>,
    buffer: cosmic_text::Buffer,
    text: String,
    pipeline: RenderPipeline,
    atlas_bind_group_layout: BindGroupLayout,
    surface_size: (u32, u32),
    atlas_bind_group: BindGroup,
}

impl TextRenderer {
    pub fn new_with_metrics(
        device: &Device,
        queue: &Queue,
        surface: &SurfaceConfiguration,
        metrics: cosmic_text::Metrics,
    ) -> Self {
        let mut font_system = FontSystem::new();
        let swash_cache = SwashCache::new();
        let buffer = cosmic_text::Buffer::new(&mut font_system, metrics);
        let atlas = GlyphAtlas::new(2048, device);

        let glyph_buffer = VertexBuffer::new(device, "Glyph vertex buffer", None);

        let shader_module =
            device.create_shader_module(include_wgsl!("../../../shaders/text.wgsl"));

        let atlas_bind_group_layout =
            device.create_bind_group_layout(&GlyphAtlas::get_bind_group_layout_desc());

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Main render pipeline layout"),
            bind_group_layouts: &[&atlas_bind_group_layout],
            push_constant_ranges: &[],
        });
        let render_pipeline = PipelineBuilder::new(device, "Main render pipeline")
            .with_shader(&shader_module)
            .add_color_target(
                surface.format,
                Some(BlendState::ALPHA_BLENDING),
                ColorWrites::ALL,
            )
            .add_vertex_layout(
                &GlyphToRender::get_buffer_attributes(0),
                std::mem::size_of::<GlyphToRender>() as u64,
                wgpu::VertexStepMode::Instance,
            )
            .with_layout(&pipeline_layout)
            .build();

        let atlas_bind_group = atlas.generate_bind_group(&atlas_bind_group_layout, queue, device);
        let surface_size = (surface.width, surface.height);

        Self {
            font_system,
            swash_cache,
            buffer,
            atlas,
            glyph_buffer,
            atlas_bind_group,
            atlas_bind_group_layout,
            pipeline: render_pipeline,
            surface_size,
            cache: Vec::new(),
            text: String::new(),
            placement_cache: HashMap::new(),
            attributes: cosmic_text::Attrs::new(),
        }
    }

    fn fill_cache(&mut self) {
        if !self.cache.is_empty() || self.text.is_empty() {
            return;
        }

        let mut glyphs_to_render = Vec::with_capacity(self.text.len());
        let mut glyphs = Vec::new();

        for line in self.buffer.layout_runs() {
            for glyph in line.glyphs {
                let glyph_id = glyph.glyph_id;
                let font_id = glyph.font_id;
                let pos = (glyph.x, glyph.y);
                let cache_key = cosmic_text::CacheKey::new(
                    font_id,
                    glyph_id,
                    glyph.font_size,
                    pos,
                    glyph.cache_key_flags,
                );

                let atlas_id = GlyphRectId::new(glyph_id, cache_key.0);
                let placement = if !self.placement_cache.contains_key(&glyph_id) {
                    let glyph_img = self
                        .swash_cache
                        .get_image_uncached(&mut self.font_system, cache_key.0)
                        .unwrap();

                    let placement = glyph_img.placement;

                    self.placement_cache.insert(glyph_id, placement);
                    glyphs.push((atlas_id, Glyph::get_atlas_image(glyph_img)));

                    placement
                } else {
                    *self.placement_cache.get(&glyph_id).unwrap()
                };

                let (width, height) = (placement.width, placement.height);
                let pos = (glyph.x.round(), line.line_y.round() + glyph.y);
                let glyph_placement = (
                    pos.0,
                    pos.1 - placement.top as f32,
                    width as f32,
                    height as f32,
                );

                let glyph_to_render = (glyph_placement, atlas_id);
                glyphs_to_render.push(glyph_to_render);
            }
        }

        if !glyphs.is_empty() {
            self.atlas.add_glyphs(glyphs.as_slice());
        }

        let new_cache = glyphs_to_render
            .iter()
            .map(|(placement, atlas_id)| {
                let atlas_glyph = self.atlas.get_glyph(&atlas_id.glyph_id).unwrap();
                self.create_glyph_to_render(*placement, atlas_glyph)
            })
            .collect();

        self.cache = new_cache;
    }

    pub fn set_text(&mut self, device: &Device, queue: &Queue, text: String) {
        if self.text == text {
            return;
        }

        self.text = text;
        self.buffer.set_text(
            &mut self.font_system,
            &self.text,
            &self.attributes,
            cosmic_text::Shaping::Basic,
        );

        self.cache.clear();
        self.fill_cache();
        self.glyph_buffer
            .write(device, queue, self.cache.as_slice());
        self.atlas_bind_group =
            self.atlas
                .generate_bind_group(&self.atlas_bind_group_layout, queue, device);
    }

    pub fn resize(&mut self, width: u32, height: u32, device: &Device, queue: &Queue) {
        self.surface_size = (width, height);
        self.buffer.set_size(
            &mut self.font_system,
            Some(width as f32),
            Some(height as f32),
        );

        self.cache.clear();
        self.fill_cache();
        self.glyph_buffer
            .write(device, queue, self.cache.as_slice());
    }

    pub fn draw(&self, render_pass: &mut RenderPass) {
        render_pass.set_pipeline(&self.pipeline);
        render_pass.set_vertex_buffer(0, self.glyph_buffer.raw_buffer().slice(..));
        render_pass.set_bind_group(0, &self.atlas_bind_group, &[]);
    }

    fn create_glyph_to_render(
        &self,
        placement: (f32, f32, f32, f32),
        atlas_glyph: &Glyph,
    ) -> GlyphToRender {
        let surface_width = self.surface_size.0 as f32;
        let surface_height = self.surface_size.1 as f32;

        let (x, y, w, h) = placement;

        let (x, y, w, h) = (
            x / surface_width * 2.0 - 1.0,
            1.0 - y / surface_height * 2.0,
            (x + w) / surface_width * 2.0 - 1.0,
            1.0 - (y + h) / surface_height * 2.0,
        );

        let atlas_size = (
            self.atlas.image.width() as f32,
            self.atlas.image.height() as f32,
        );

        GlyphToRender::new(x, y, w, h, atlas_glyph, atlas_size)
    }
}

#[derive(Debug, Clone, Copy, Pod, Zeroable)]
#[repr(C)]
pub struct GlyphToRender {
    /// x, y postion of the glyph at the screen in screen coordinates
    pos: [f32; 4],
    /// x, y, with and height of the glyph at the atlas in pixel coordinates
    atlas_uv: [f32; 4],
    format: f32,
}

impl GlyphToRender {
    fn new(x: f32, y: f32, w: f32, h: f32, glyph: &Glyph, atlas_size: (f32, f32)) -> Self {
        let (atlas_width, atlas_height) = atlas_size;
        let [x_uv, y_uv, w_uv, h_uv] = glyph.atlas_uv();
        let atlas_uv = [
            x_uv / atlas_width,
            y_uv / atlas_height,
            w_uv / atlas_width,
            h_uv / atlas_height,
        ];
        let format = match glyph.format {
            atlas::GlyphImageFormat::GrayScale => 0.0,
            atlas::GlyphImageFormat::Color => 1.0,
        };

        Self {
            pos: [x, y, w, h],
            atlas_uv,
            format,
        }
    }

    pub fn get_buffer_attributes(start_idx: u32) -> [VertexAttribute; 3] {
        [
            VertexAttribute {
                format: wgpu::VertexFormat::Float32x4,
                offset: 0,
                shader_location: start_idx,
            },
            VertexAttribute {
                format: wgpu::VertexFormat::Float32x4,
                offset: std::mem::size_of::<[f32; 4]>() as u64,
                shader_location: start_idx + 1,
            },
            VertexAttribute {
                format: wgpu::VertexFormat::Float32,
                offset: std::mem::size_of::<[f32; 4]>() as u64 * 2,
                shader_location: start_idx + 2,
            },
        ]
    }
}
