let PI: f32 = 3.14159265359;

struct Uniforms {
    perspective: mat4x4<f32>;
    orthographic: mat4x4<f32>;
    total_time: f32;
    delta_time: f32;
};

struct MeshVertex {
    m0: vec4<f32>;
    m1: vec4<f32>;
    m2: vec4<f32>;
};

struct MeshVertices {
    vertices: [[stride(48)]] array<MeshVertex>;
};

struct LineIndices {
    indices: [[stride(4)]] array<u32>;
};

[[group(0), binding(0)]]
var<uniform> r_uniforms: Uniforms;

[[group(1), binding(0)]]
var<storage, read> mesh_vertices: MeshVertices;

[[group(1), binding(1)]]
var<storage, read> line_indices: LineIndices;

struct VertexInput {
    [[builtin(vertex_index)]] v_index: u32;
    [[location(0)]] position: vec3<f32>;
    [[location(1)]] end: f32;
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
    [[builtin(instance_index)]] instance: u32,
    in: VertexInput
) -> VertexOutput {
    let i0 = instance * u32(2);
    let i1 = i0 + u32(1);

    let i0 = line_indices.indices[i0];
    let i1 = line_indices.indices[i1];

    let v0 = mesh_vertices.vertices[i0];
    let v0_pos = v0.m0.xyz;
    let v0_surface_color = vec3<f32>(v0.m0.w, v0.m1.xy);
    let v0_line_color = vec3<f32>(v0.m1.zw, v0.m2.x);
    let v0_intensity = v0.m2.y;
    let v0_delta_intensity = v0.m2.z;

    let v1 = mesh_vertices.vertices[i1];
    let v1_pos = v1.m0.xyz;
    let v1_surface_color = vec3<f32>(v1.m0.w, v1.m1.xy);
    let v1_line_color = vec3<f32>(v1.m1.zw, v1.m2.x);
    let v1_intensity = v1.m2.y;
    let v1_delta_intensity = v1.m2.z;

    let v0 = r_uniforms.perspective * vec4<f32>(v0_pos, 1.0);
    let v1 = r_uniforms.perspective * vec4<f32>(v1_pos, 1.0);

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
    output.color = mix(v0_line_color, v1_line_color, in.end);
    output.intensity = mix(v0_intensity, v1_intensity, in.end);
    output.delta_intensity = mix(v0_delta_intensity, v1_delta_intensity, in.end);
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
