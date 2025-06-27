use cosmic_text::{SwashContent, SwashImage};
use image::{DynamicImage, GrayImage, ImageBuffer, RgbaImage};
use rectangle_pack::{GroupedRectsToPlace, RectToInsert, TargetBin};
use std::collections::{BTreeMap, HashMap};
use wgpu::{
    BindGroup, BindGroupEntry, BindGroupLayout, Device, Queue, TexelCopyBufferLayout, TextureUsages,
};

#[derive(Debug, Clone, Copy)]
pub struct Glyph {
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,
    pub format: GlyphImageFormat,
}

#[derive(Debug, Clone, Copy)]
pub enum GlyphImageFormat {
    Color,
    GrayScale,
}

impl Glyph {
    pub fn new(x: u32, y: u32, width: u32, height: u32, format: GlyphImageFormat) -> Self {
        Self {
            x,
            y,
            width,
            height,
            format,
        }
    }

    pub fn atlas_uv(&self) -> [f32; 4] {
        [
            self.x as f32,
            self.y as f32,
            self.width as f32,
            self.height as f32,
        ]
    }

    pub fn get_atlas_image(img: SwashImage) -> DynamicImage {
        let (width, height) = (img.placement.width, img.placement.height);
        match img.content {
            SwashContent::Color => {
                let img = RgbaImage::from_raw(width, height, img.data).unwrap();
                DynamicImage::ImageRgba8(img)
            }
            _ => {
                let img = GrayImage::from_vec(width, height, img.data).unwrap();
                DynamicImage::ImageLuma8(img)
            }
        }
    }
}

#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub struct GlyphRectId {
    pub cache_key: cosmic_text::CacheKey,
}

impl GlyphRectId {
    pub fn new(cache_key: cosmic_text::CacheKey) -> Self {
        Self { cache_key }
    }
}

#[derive(Debug)]
pub struct GlyphAtlas {
    pub image: RgbaImage,
    pub glyphs: HashMap<cosmic_text::CacheKey, Glyph>,
    sampler: wgpu::Sampler,
    texture: wgpu::Texture,
    targets: BTreeMap<u8, TargetBin>,
}

impl GlyphAtlas {
    pub fn new(size: u32, device: &Device) -> Self {
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("Glyph atlas texture"),
            size: wgpu::Extent3d {
                width: size,
                height: size,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8Unorm,
            usage: TextureUsages::TEXTURE_BINDING | TextureUsages::COPY_DST,
            view_formats: &[],
        });

        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("Glyph atlas sampler"),
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::MirrorRepeat,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Nearest,
            mipmap_filter: wgpu::FilterMode::Nearest,
            ..Default::default()
        });

        let mut targets = BTreeMap::new();
        targets.insert(0, TargetBin::new(size, size, 1));

        Self {
            image: ImageBuffer::new(size, size),
            texture,
            sampler,
            targets,
            glyphs: HashMap::new(),
        }
    }

    pub fn add_glyphs(&mut self, glyphs: &[(GlyphRectId, DynamicImage)]) {
        let new_glyphs: Vec<_> = glyphs
            .iter()
            .filter(|(rect_id, _)| !self.glyphs.contains_key(&rect_id.cache_key))
            .collect();

        if new_glyphs.is_empty() {
            return;
        }

        let glyphs_with_rgba: Vec<_> = new_glyphs
            .iter()
            .map(|(rect_id, img)| {
                let format = match img.color() {
                    image::ColorType::Rgba8 => GlyphImageFormat::Color,
                    _ => GlyphImageFormat::GrayScale,
                };
                (*rect_id, img.to_rgba8(), format)
            })
            .collect();

        let mut rects_to_place: GroupedRectsToPlace<GlyphRectId, u16> = GroupedRectsToPlace::new();
        for (rect_id, img, _) in &glyphs_with_rgba {
            rects_to_place.push_rect(
                *rect_id,
                None,
                RectToInsert::new(img.width() + 2, img.height() + 2, 1),
            );
        }

        // FIXME: Should create another target and texture when out of space
        // maybe other option is expand the atlas dimensions and create another atlas on backup if is too big?
        let packing_result = rectangle_pack::pack_rects(
            &rects_to_place,
            &mut self.targets,
            &rectangle_pack::volume_heuristic,
            &rectangle_pack::contains_smallest_box,
        )
        .unwrap();

        let id_to_index: HashMap<_, _> = glyphs_with_rgba
            .iter()
            .enumerate()
            .map(|(i, (id, _, _))| (id, i))
            .collect();

        for (rect_id, (_, location)) in packing_result.packed_locations() {
            let (_, img, format) = &glyphs_with_rgba[*id_to_index.get(rect_id).unwrap()];
            let (x, y) = (location.x(), location.y());

            for (row, img_row) in img.rows().enumerate() {
                let atlas_y = y + row as u32;
                if atlas_y >= self.image.height() {
                    break;
                }

                for (col, pixel) in img_row.enumerate() {
                    let atlas_x = x + col as u32;
                    if atlas_x >= self.image.width() {
                        break;
                    }

                    self.image.put_pixel(atlas_x, atlas_y, *pixel);
                }
            }

            self.glyphs.insert(
                rect_id.cache_key,
                Glyph::new(x, y, location.width() - 2, location.height() - 2, *format),
            );
        }
    }

    pub fn get_glyph(&self, id: &cosmic_text::CacheKey) -> Option<&Glyph> {
        self.glyphs.get(id)
    }

    pub fn get_bind_group_layout_desc() -> wgpu::BindGroupLayoutDescriptor<'static> {
        wgpu::BindGroupLayoutDescriptor {
            label: Some("Atlas bind group layout"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    count: None,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    count: None,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                },
            ],
        }
    }

    pub fn generate_bind_group(
        &self,
        layout: &BindGroupLayout,
        queue: &Queue,
        device: &Device,
    ) -> BindGroup {
        queue.write_texture(
            self.texture.as_image_copy(),
            self.image.as_raw(),
            TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(self.image.width() * 4),
                rows_per_image: Some(self.image.height()),
            },
            wgpu::Extent3d {
                width: self.image.width(),
                height: self.image.height(),
                depth_or_array_layers: 1,
            },
        );
        let texture_view = self
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());

        device.create_bind_group(&wgpu::BindGroupDescriptor {
            entries: &[
                BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&texture_view),
                },
                BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&self.sampler),
                },
            ],
            label: Some("Glyph atlas bind group"),
            layout,
        })
    }
}
