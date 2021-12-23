[[group(0), binding(0)]]
var r_phosphor: texture_2d<f32>;

[[group(0), binding(2)]]
var r_sampler: sampler;

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
    let phosphor = textureSample(r_phosphor, r_sampler, in.uv);

    let color = phosphor.rgb;
    let black = vec3<f32>(0.0, 0.0, 0.0);
    let white = vec3<f32>(1.0, 1.0, 1.0);

    let fac = max(color - white, black);
    let fac = fac.r + fac.g + fac.b;

    let color = mix(color, white, clamp(fac, 0.0, 1.0));

    return vec4<f32>(color, 1.0);
}
