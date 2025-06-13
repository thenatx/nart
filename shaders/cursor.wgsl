struct CursorInstance {
  @location(0) pos: vec2<f32>,
  @location(1) size: vec2<f32>,
}

struct VertexOutput {
  @builtin(position) position: vec4<f32>,
  @location(0) color: vec4<f32>
}

@vertex
fn vs_main(
    @builtin(vertex_index) vertex_idx: u32,
    instance: CursorInstance
) -> VertexOutput {
    let size = instance.size;
    let pos = instance.pos;
    let positions = array<vec2<f32>, 6>(
        vec2(pos.x, pos.y),
        vec2(pos.x, pos.y - size.y),
        vec2(pos.x + size.x, pos.y),
        vec2(pos.x + size.x, pos.y),
        vec2(pos.x, pos.y - size.y),
        vec2(pos.x + size.x, pos.y - size.y),
    );

    let position = positions[vertex_idx];
    var out: VertexOutput;

    // TODO: Implement actual color config for the cursor
    out.color = vec4f(1.0, 1.0, 1.0, 1.0);
    out.position = vec4<f32>(position, 0.0, 1.0);

    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
  return in.color;
}
