pub mod atlas;
pub mod cursor;

use bytemuck::{Pod, Zeroable};
use cosmic_text::{Buffer, CacheKey, FontSystem, LayoutGlyph, Shaping, SwashCache};
use image::DynamicImage;
use wgpu::{
    include_wgsl, BindGroup, BindGroupLayout, BlendState, ColorWrites, Device, Queue, RenderPass,
    RenderPipeline, SurfaceConfiguration, VertexAttribute,
};

use atlas::{Glyph, GlyphAtlas, GlyphRectId};

use super::{buffer::VertexBuffer, pipeline::PipelineBuilder};

pub struct TextRenderer {
    buffer: cosmic_text::Buffer,
    font_system: FontSystem,
    atlas: GlyphAtlas,
    glyph_buffer: VertexBuffer<GlyphToRender>,
    cache: Vec<GlyphToRender>,
    swash_cache: SwashCache,
    attributes: cosmic_text::Attrs<'static>,
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
        let buffer = Buffer::new(&mut font_system, metrics);

        let atlas = GlyphAtlas::new(2048, device);
        let glyph_buffer = VertexBuffer::new(device, "Glyph vertex buffer", None);

        let shader_module =
            device.create_shader_module(include_wgsl!("../../../shaders/text.wgsl"));

        let atlas_bind_group_layout =
            device.create_bind_group_layout(&GlyphAtlas::get_bind_group_layout_desc());

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Text render pipeline layout"),
            bind_group_layouts: &[&atlas_bind_group_layout],
            push_constant_ranges: &[],
        });

        let render_pipeline = PipelineBuilder::new(device, "Text render pipeline")
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

        let mut features = cosmic_text::FontFeatures::new();
        features.enable(cosmic_text::FeatureTag::KERNING);

        let attributes = cosmic_text::Attrs::new()
            .family(cosmic_text::Family::Monospace)
            .font_features(features);

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
            attributes,
        }
    }

    // TODO: Improve glyph positioning
    fn fill_cache(&mut self) {
        if !self.cache.is_empty() || self.buffer.lines.is_empty() {
            return;
        }

        if !self.buffer.redraw() {
            return;
        }

        let mut new_cache = Vec::new();

        for line in self.buffer.layout_runs() {
            for glyph in line.glyphs {
                let pos = (glyph.x, glyph.y);
                let cache_key = CacheKey::new(
                    glyph.font_id,
                    glyph.glyph_id,
                    glyph.font_size,
                    pos,
                    glyph.cache_key_flags,
                );

                let glyph_img = self
                    .swash_cache
                    .get_image_uncached(&mut self.font_system, cache_key.0)
                    .expect("Failed to get glyph image - font not found");

                let placement = glyph_img.placement;
                let glyph_placement = self.calculate_glyph_position(glyph, &line, placement);
                let atlas_id = self.create_atlas_id(cache_key.0);

                let color = glyph
                    .color_opt
                    .unwrap_or(cosmic_text::Color::rgb(255, 255, 255));
                new_cache.push((glyph_placement, color, atlas_id));
            }
        }

        self.cache = new_cache
            .iter()
            .filter_map(|(placement, color, atlas_id)| {
                self.atlas
                    .get_glyph(&atlas_id.cache_key)
                    .map(|atlas_glyph| self.create_glyph_to_render(*placement, atlas_glyph, *color))
            })
            .collect();
    }

    pub fn add_text(&mut self, device: &Device, queue: &Queue, content: &[StyledCharacter]) {
        if content.is_empty() {
            return;
        }

        self.buffer.set_rich_text(
            &mut self.font_system,
            content.iter().map(|i| {
                let attrs = self.attributes.clone().color(i.color.into());
                (i.character.as_str(), attrs)
            }),
            &self.attributes,
            Shaping::Advanced,
            None,
        );
        let runs = self.buffer.layout_runs().collect::<Vec<_>>();
        let new_glyphs = Self::process_glyphs(
            runs.as_slice(),
            &mut self.font_system,
            &mut self.swash_cache,
        );

        if !new_glyphs.is_empty() {
            self.atlas.add_glyphs(new_glyphs.as_slice());
        }

        self.cache.clear();
        self.fill_cache();

        self.glyph_buffer.write(device, queue, &self.cache);
        self.atlas_bind_group =
            self.atlas
                .generate_bind_group(&self.atlas_bind_group_layout, queue, device);
    }

    fn process_glyphs(
        runs: &[cosmic_text::LayoutRun],
        font_system: &mut FontSystem,
        swash_cache: &mut SwashCache,
    ) -> Vec<(GlyphRectId, DynamicImage)> {
        runs.iter()
            .flat_map(|line| line.glyphs.iter())
            .filter_map(|glyph| {
                let pos = (glyph.x, glyph.y);
                let cache_key = cosmic_text::CacheKey::new(
                    glyph.font_id,
                    glyph.glyph_id,
                    glyph.font_size,
                    pos,
                    glyph.cache_key_flags,
                );
                let id = GlyphRectId::new(cache_key.0);
                swash_cache
                    .get_image_uncached(font_system, cache_key.0)
                    .map(|img| (id, Glyph::get_atlas_image(img)))
            })
            .collect()
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
        render_pass.draw(0..6, 0..self.cache.len() as u32);
    }

    pub fn get_glyph_size(&mut self) -> Option<(f32, f32)> {
        let width = if let Some(width) = self.buffer.monospace_width() {
            width
        } else {
            let width = self.buffer.metrics().font_size * 0.6;
            self.buffer
                .set_monospace_width(&mut self.font_system, Some(width));
            width
        };

        if let Some(run) = self.buffer.layout_runs().last() {
            return Some((width, run.line_height));
        };

        None
    }

    fn calculate_glyph_position(
        &self,
        glyph: &LayoutGlyph,
        line: &cosmic_text::LayoutRun,
        placement: cosmic_text::Placement,
    ) -> (f32, f32, f32, f32) {
        let x = glyph.x.round();
        let y = line.line_y.round() + glyph.y;
        let width = placement.width as f32;
        let height = placement.height as f32;

        (
            x + placement.left as f32,
            y - placement.top as f32,
            width,
            height,
        )
    }

    fn create_atlas_id(&self, glyph_cache_key: cosmic_text::CacheKey) -> GlyphRectId {
        GlyphRectId::new(glyph_cache_key)
    }

    fn create_glyph_to_render(
        &self,
        placement: (f32, f32, f32, f32),
        atlas_glyph: &Glyph,
        color: cosmic_text::Color,
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

        GlyphToRender::new(x, y, w, h, atlas_glyph, atlas_size, color)
    }
}

#[derive(Debug, Clone, Copy, Pod, Zeroable)]
#[repr(C)]
pub struct GlyphToRender {
    /// x, y postion of the glyph at the screen in screen coordinates
    pos: [f32; 4],
    /// x, y, with and height of the glyph at the atlas in pixel coordinates
    atlas_uv: [f32; 4],
    color: [f32; 4],
    format: f32,
}

impl GlyphToRender {
    fn new(
        x: f32,
        y: f32,
        w: f32,
        h: f32,
        glyph: &Glyph,
        atlas_size: (f32, f32),
        color: cosmic_text::Color,
    ) -> Self {
        let (atlas_width, atlas_height) = atlas_size;
        let [x_uv, y_uv, w_uv, h_uv] = glyph.atlas_uv();
        let atlas_uv = [
            x_uv / atlas_width,
            y_uv / atlas_height,
            w_uv / atlas_width,
            h_uv / atlas_height,
        ];

        let color = color
            .as_rgba()
            .as_slice()
            .iter()
            .map(|&c| c as f32 / 255.0)
            .collect::<Vec<f32>>()
            .try_into()
            .expect("Color should be RGBA with 4 components");

        let format = match glyph.format {
            atlas::GlyphImageFormat::GrayScale => 0.0,
            atlas::GlyphImageFormat::Color => 1.0,
        };

        Self {
            pos: [x, y, w, h],
            atlas_uv,
            format,
            color,
        }
    }

    pub fn get_buffer_attributes(start_idx: u32) -> [VertexAttribute; 4] {
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
                format: wgpu::VertexFormat::Float32x4,
                offset: std::mem::size_of::<[f32; 4]>() as u64 * 2,
                shader_location: start_idx + 2,
            },
            VertexAttribute {
                format: wgpu::VertexFormat::Float32,
                offset: std::mem::size_of::<[f32; 4]>() as u64 * 3,
                shader_location: start_idx + 3,
            },
        ]
    }
}

pub struct StyledCharacter {
    character: String,
    color: crate::graphics::Color,
}

impl StyledCharacter {
    pub fn new(character: String, color: crate::graphics::Color) -> Self {
        Self { character, color }
    }
}
