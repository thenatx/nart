use pollster::FutureExt;
use wgpu::{
    Backends, Device, DeviceDescriptor, Instance, InstanceFlags, Queue, RequestAdapterOptions,
    Surface, SurfaceConfiguration, TextureUsages,
};

pub mod buffer;
pub mod pipeline;
pub mod renderer;
pub mod text;

struct WgpuContext<'a> {
    surface: Surface<'a>,
    surf_config: SurfaceConfiguration,
    device: Device,
    queue: Queue,
}

impl<'a> WgpuContext<'a> {
    fn new<T>(target: &T, width: u32, height: u32) -> Self
    where
        T: Into<wgpu::SurfaceTarget<'a>> + Clone,
    {
        let instance = Instance::new(&wgpu::InstanceDescriptor {
            backends: Backends::PRIMARY,
            flags: InstanceFlags::empty(),
            backend_options: wgpu::BackendOptions::from_env_or_default(),
        });

        let surface = instance.create_surface(target.clone()).unwrap();
        let adapter_ops = RequestAdapterOptions {
            compatible_surface: Some(&surface),
            force_fallback_adapter: false,
            power_preference: wgpu::PowerPreference::LowPower,
        };

        let adapter = instance.request_adapter(&adapter_ops).block_on().unwrap();
        let surface_caps = surface.get_capabilities(&adapter);

        let surface_format = surface_caps
            .formats
            .iter()
            .find(|f| f.is_srgb())
            .copied()
            .unwrap_or(surface_caps.formats[0]);

        let surf_config = SurfaceConfiguration {
            width,
            height,
            alpha_mode: wgpu::CompositeAlphaMode::Auto,
            present_mode: wgpu::PresentMode::AutoVsync,
            usage: TextureUsages::RENDER_ATTACHMENT,
            desired_maximum_frame_latency: 2,
            format: surface_format,
            view_formats: Vec::new(),
        };

        let (device, queue) = adapter
            .request_device(&DeviceDescriptor {
                label: Some("Main wgpu device"),
                memory_hints: wgpu::MemoryHints::Performance,
                required_features: wgpu::Features::empty(),
                required_limits: wgpu::Limits::downlevel_defaults(),
                trace: wgpu::Trace::Off,
            })
            .block_on()
            .unwrap();

        surface.configure(&device, &surf_config);
        Self {
            surface,
            surf_config,
            device,
            queue,
        }
    }

    fn surface_resize(&mut self, width: u32, height: u32) {
        self.surf_config.width = width;
        self.surf_config.height = height;

        self.surface.configure(&self.device, &self.surf_config);
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Color {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,
}

impl Color {
    pub fn new(r: u8, g: u8, b: u8, a: u8) -> Self {
        Self { r, g, b, a }
    }
}

impl From<Color> for wgpu::Color {
    fn from(val: Color) -> Self {
        let [r, g, b, a] = [
            val.r as f64 / 255.0,
            val.g as f64 / 255.0,
            val.b as f64 / 255.0,
            val.a as f64 / 255.0,
        ];

        wgpu::Color { r, g, b, a }
    }
}

impl From<Color> for cosmic_text::Color {
    fn from(value: Color) -> Self {
        cosmic_text::Color::rgb(value.r, value.g, value.b)
    }
}
