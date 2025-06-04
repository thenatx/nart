use wgpu::{
    BlendState, ColorTargetState, Device, FragmentState, MultisampleState,
    PipelineCompilationOptions, PipelineLayout, PrimitiveState, RenderPipeline,
    RenderPipelineDescriptor, ShaderModule, TextureFormat, VertexAttribute, VertexBufferLayout,
    VertexStepMode,
};

pub struct PipelineBuilder<'a> {
    device: &'a Device,
    shader: Option<&'a ShaderModule>,
    layout: Option<&'a PipelineLayout>,
    vertex_layouts: Vec<VertexBufferLayout<'a>>,
    targets: Vec<Option<ColorTargetState>>,
    primitive: PrimitiveState,
    label: &'a str,
}

impl<'a> PipelineBuilder<'a> {
    pub fn new(device: &'a Device, label: &'a str) -> Self {
        Self {
            label,
            device,
            shader: None,
            layout: None,
            vertex_layouts: Vec::new(),
            targets: Vec::new(),
            primitive: PrimitiveState::default(),
        }
    }

    pub fn with_shader(mut self, shader: &'a ShaderModule) -> Self {
        self.shader = Some(shader);
        self
    }

    pub fn with_layout(mut self, layout: &'a PipelineLayout) -> Self {
        self.layout = Some(layout);
        self
    }

    pub fn add_vertex_layout(
        mut self,
        attributes: &'a [VertexAttribute],
        stride: u64,
        step_mode: VertexStepMode,
    ) -> Self {
        self.vertex_layouts.push(VertexBufferLayout {
            array_stride: stride,
            step_mode,
            attributes,
        });
        self
    }

    pub fn add_color_target(
        mut self,
        format: TextureFormat,
        blend: Option<BlendState>,
        write_mask: wgpu::ColorWrites,
    ) -> Self {
        self.targets.push(Some(ColorTargetState {
            format,
            blend,
            write_mask,
        }));
        self
    }

    pub fn build(self) -> RenderPipeline {
        let shader = self.shader.expect("Shader module must be provided");
        let layout = self.layout.expect("Pipeline layout must be provided");

        self.device
            .create_render_pipeline(&RenderPipelineDescriptor {
                label: Some(self.label),
                layout: Some(layout),
                vertex: wgpu::VertexState {
                    module: shader,
                    entry_point: None,
                    buffers: &self.vertex_layouts,
                    compilation_options: PipelineCompilationOptions::default(),
                },
                fragment: Some(FragmentState {
                    module: shader,
                    entry_point: None,
                    targets: &self.targets,
                    compilation_options: PipelineCompilationOptions::default(),
                }),
                primitive: self.primitive,
                depth_stencil: None,
                multisample: MultisampleState::default(),
                multiview: None,
                cache: None,
            })
    }
}
