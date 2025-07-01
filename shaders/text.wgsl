struct GlyphInstance {
    @location(0) pos: vec4<f32>,
    @location(1) uv: vec4<f32>,
    @location(2) color: vec4<f32>,
    @location(3) format: f32,
};

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
    @location(1) color: vec4<f32>,
    @location(2) format: u32,
};

@vertex
fn vs_main(
    @builtin(vertex_index) vert_idx: u32,
    instance: GlyphInstance,
) -> VertexOutput {
    let uv_coords = vec4<f32>(
        instance.uv.x,
        instance.uv.y,
        instance.uv.x + instance.uv.z,
        instance.uv.y + instance.uv.w
    );

    var out: VertexOutput;

    let xy = vec2<f32>(
        mix(instance.pos.x, instance.pos.z, f32((vert_idx & 1) != 0)),
        mix(instance.pos.y, instance.pos.w, f32((vert_idx & 2) != 0))
    );

    let uv = vec2<f32>(
        mix(uv_coords.x, uv_coords.z, f32((vert_idx & 1) != 0)),
        mix(uv_coords.y, uv_coords.w, f32((vert_idx & 2) != 0))
    );

    let ccw_indices = array<u32, 6>(0, 2, 1, 1, 2, 3);
    let idx = ccw_indices[vert_idx];

    out.position = vec4<f32>(
        mix(instance.pos.x, instance.pos.z, f32((idx & 1) != 0)),
        mix(instance.pos.y, instance.pos.w, f32((idx & 2) != 0)),
        0.0,
        1.0
    );

    out.uv = vec2<f32>(
        mix(uv_coords.x, uv_coords.z, f32((idx & 1) != 0)),
        mix(uv_coords.y, uv_coords.w, f32((idx & 2) != 0))
    );
    out.color = instance.color;
    out.format = u32(instance.format);
    return out;
}

@group(0) @binding(0) var atlas_texture: texture_2d<f32>;
@group(0) @binding(1) var atlas_sampler: sampler;

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
  let color = textureSample(atlas_texture, atlas_sampler, in.uv);
  switch in.format {
    case 0: {
      return vec4(in.color.rgb, color.r);
    }
    default: {
      return color;
    }
  }
}
