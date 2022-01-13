let PI: f32 = 3.14159265359;

struct Uniforms {
    perspective: mat4x4<f32>;
    orthographic: mat4x4<f32>;
    total_time: f32;
    delta_time: f32;
};

struct TriangleMeshInstance {
    pos: vec4<f32>;
    rot: vec4<f32>;
    scale: vec4<f32>;
};

struct TriangleMeshInstances {
    instances: [[stride(48)]] array<TriangleMeshInstance>;
};

[[group(0), binding(0)]]
var<uniform> r_uniforms: Uniforms;

[[group(1), binding(1)]]
var<storage, read> triangle_mesh_instances: TriangleMeshInstances;

struct VertexInput {
    [[builtin(vertex_index)]] v_index: u32;
    [[location(0)]] position: vec3<f32>;
    [[location(1)]] surface_color: vec3<f32>;
    [[location(2)]] line_color: vec3<f32>;
    [[location(3)]] intensity: f32;
    [[location(4)]] delta_intensity: f32;
};

struct VertexOutput {
    [[builtin(position)]] position: vec4<f32>;
    [[location(0)]] color: vec3<f32>;
    [[location(1)]] intensity: f32;
    [[location(2)]] delta_intensity: f32;
};

[[stage(vertex)]]
fn vs_main(
    [[builtin(instance_index)]] instance: u32,
    in: VertexInput
) -> VertexOutput {
    let instance = triangle_mesh_instances.instances[instance];
    let instance_pos = instance.pos.xyz;
    let instance_scale = instance.scale.xyz;

    let pos = instance_pos + (in.position * instance_scale);

    var output: VertexOutput;
    output.position = r_uniforms.perspective * vec4<f32>(pos, 1.0);
    output.color = in.surface_color;
    output.intensity = in.intensity;
    output.delta_intensity = in.delta_intensity;
    return output;
}

[[stage(fragment)]]
fn fs_main(
    in: VertexOutput,
) -> [[location(0)]] vec4<f32> {
    return vec4<f32>(in.color * in.intensity, in.delta_intensity);
}
