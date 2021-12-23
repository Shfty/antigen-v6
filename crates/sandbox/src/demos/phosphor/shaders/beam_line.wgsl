let PI: f32 = 3.14159265359;

struct Uniforms {
    perspective: mat4x4<f32>;
    orthographic: mat4x4<f32>;
    total_time: f32;
    delta_time: f32;
};

[[group(0), binding(0)]]
var<uniform> r_uniforms: Uniforms;

struct VertexInput {
    [[builtin(vertex_index)]] v_index: u32;
    [[location(0)]] position: vec3<f32>;
    [[location(1)]] end: f32;
    [[location(2)]] v0: vec3<f32>;
    [[location(3)]] v0_surface_color: vec3<f32>;
    [[location(4)]] v0_line_color: vec3<f32>;
    [[location(5)]] v0_intensity: f32;
    [[location(6)]] v0_delta_intensity: f32;
    [[location(7)]] v1: vec3<f32>;
    [[location(8)]] v1_surface_color: vec3<f32>;
    [[location(9)]] v1_line_color: vec3<f32>;
    [[location(10)]] v1_intensity: f32;
    [[location(11)]] v1_delta_intensity: f32;
};

struct VertexOutput {
    [[builtin(position)]] position: vec4<f32>;
    [[location(0)]] color: vec3<f32>;
    [[location(1)]] intensity: f32;
    [[location(2)]] delta_intensity: f32;
};

fn rotate(v: vec3<f32>, angle: f32) -> vec3<f32> {
    let cs = cos(angle);
    let sn = sin(angle);
    return vec3<f32>(
        v.x * cs - v.y * sn,
        v.x * sn + v.y * cs,
        v.z,
    );
}

[[stage(vertex)]]
fn vs_main(
    in: VertexInput
) -> VertexOutput {
    let v0 = r_uniforms.perspective * vec4<f32>(in.v0, 1.0);
    let v1 = r_uniforms.perspective * vec4<f32>(in.v1, 1.0);

    let v0 = v0.xyz / v0.w;
    let v1 = v1.xyz / v1.w;

    var delta = v1 - v0;

    let delta_norm = normalize(delta);

    var angle = 0.0;
    if(length(delta_norm) > 0.0) {
        angle = atan2(delta_norm.y, delta_norm.x);
    }

    let vert = in.position.xyz;
    let vert = rotate(vert, angle);
    let vert = (r_uniforms.orthographic * vec4<f32>(vert, 1.0)).xyz;

    let pos = vert + mix(v0, v1, in.end);

    var output: VertexOutput;
    output.position = vec4<f32>(pos, 1.0);
    output.color = mix(in.v0_line_color, in.v1_line_color, in.end);
    output.intensity = mix(in.v0_intensity, in.v1_intensity, in.end);
    output.delta_intensity = mix(in.v0_delta_intensity, in.v1_delta_intensity, in.end);
    return output;
}

struct FragmentOutput {
    [[location(0)]] color: vec4<f32>;
};

[[stage(fragment)]]
fn fs_main(
    in: VertexOutput,
) -> FragmentOutput {
    var out: FragmentOutput;
    out.color = vec4<f32>(in.color * in.intensity, in.delta_intensity);
    return out;
}
