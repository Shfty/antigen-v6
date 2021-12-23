struct Uniforms {
    perspective: mat4x4<f32>;
    orthographic: mat4x4<f32>;
    total: f32;
    delta: f32;
};

[[group(0), binding(0)]]
var<uniform> r_uniforms: Uniforms;

[[group(1), binding(0)]]
var r_back_buffer: texture_2d<f32>;

[[group(1), binding(1)]]
var r_beam_buffer: texture_2d<f32>;

[[group(1), binding(2)]]
var r_linear_sampler: sampler;

struct VertexOutput {
    [[builtin(position)]] position: vec4<f32>;
    [[location(0)]] uv: vec2<f32>;
};

[[stage(vertex)]]
fn vs_main([[builtin(vertex_index)]] vertex_index: u32) -> VertexOutput {
    let x: f32 = f32(i32(vertex_index & 1u) << 2u) - 1.0;
    let y: f32 = f32(i32(vertex_index & 2u) << 1u) - 1.0;
    var output: VertexOutput;
    output.position = vec4<f32>(x, -y, 0.0, 1.0);
    output.uv = vec2<f32>(x + 1.0, y + 1.0) * 0.5;
    return output;
}

[[stage(fragment)]]
fn fs_main(in: VertexOutput) -> [[location(0)]] vec4<f32> {
    // Unpack backbuffer fragment
    let back = textureSample(r_back_buffer, r_linear_sampler, in.uv);
    let back_color = back.rgb;
    let back_delta = back.a;

    // Integrate intensity
    let back_color = clamp(back_color + vec3<f32>(back_delta) * r_uniforms.delta, vec3<f32>(0.0), vec3<f32>(8.0));

    // Unpack beam fragment
    let beam = textureSample(r_beam_buffer, r_linear_sampler, in.uv);
    let beam_color = beam.rgb;
    let delta = beam.a;

    let color = max(back_color, beam_color);

    return vec4<f32>(color, delta);
}
