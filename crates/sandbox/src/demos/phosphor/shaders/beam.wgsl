// Numeric constants
let PI: f32 = 3.14159265359;

// Quaternion functionality
struct Quaternion {
    x: f32;
    y: f32;
    z: f32;
    w: f32;
};

fn quat_from_vec4(v: vec4<f32>) -> Quaternion {
    return Quaternion(
        v.x,
        v.y,
        v.z,
        v.w,
    );
}

fn quat_inv(q: Quaternion) -> Quaternion {
    return Quaternion(-q.x, -q.y, -q.z, q.w);
}

fn quat_dot(q1: Quaternion, q2: Quaternion) -> Quaternion {
    let q1_xyz = vec3<f32>(q1.x, q1.y, q1.z);
    let q2_xyz = vec3<f32>(q2.x, q2.y, q2.z);

    let scalar = q1.w * q2.w - dot(q1_xyz, q2_xyz);
    let v = cross(q1_xyz, q2_xyz) + q1.w * q2_xyz + q2.w * q1_xyz;

    return Quaternion(v.x, v.y, v.z, scalar);
}

fn quat_mul(q: Quaternion, v: vec3<f32>) -> vec3<f32> {
    let r = quat_dot(q, quat_dot(Quaternion(v.x, v.y, v.z, 0.0), quat_inv(q)));
    return vec3<f32>(r.x, r.y, r.z);
}

// 2D rotation
fn rotate(v: vec3<f32>, angle: f32) -> vec3<f32> {
    let cs = cos(angle);
    let sn = sin(angle);
    return vec3<f32>(
        v.x * cs - v.y * sn,
        v.x * sn + v.y * cs,
        v.z,
    );
}

// Buffer structs
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

struct TriangleMeshInstance {
    pos: vec4<f32>;
    rot: Quaternion;
    scale: vec4<f32>;
};

struct TriangleMeshInstances {
    instances: [[stride(48)]] array<TriangleMeshInstance>;
};

struct LineIndices {
    indices: [[stride(4)]] array<u32>;
};

struct LineMesh {
    vertex_offset: u32;
    vertex_count: u32;
    index_offset: u32;
    index_count: u32;
};

struct LineMeshes {
    meshes: [[stride(16)]] array<LineMesh>;
};

struct LineMeshInstance {
    pos: vec3<f32>;
    mesh_id: u32;
    rot: Quaternion;
    scale: vec3<f32>;
};

struct LineMeshInstances {
    instances: [[stride(48)]] array<LineMeshInstance>;
};

struct LineInstance {
    mesh_instance_id: u32;
    line_index: u32;
};

struct LineInstances {
    instances: [[stride(8)]] array<LineInstance>;
};

// Shader I/O
struct TriangleVertexInput {
    [[builtin(vertex_index)]] v_index: u32;
    [[location(0)]] position: vec3<f32>;
    [[location(1)]] surface_color: vec3<f32>;
    [[location(2)]] line_color: vec3<f32>;
    [[location(3)]] intensity: f32;
    [[location(4)]] delta_intensity: f32;
};

struct LineVertexInput {
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

struct FragmentOutput {
    [[location(0)]] color: vec4<f32>;
};

// Data bindings
[[group(0), binding(0)]]
var<uniform> r_uniforms: Uniforms;

[[group(1), binding(0)]]
var<storage, read> mesh_vertices: MeshVertices;

[[group(1), binding(1)]]
var<storage, read> triangle_mesh_instances: TriangleMeshInstances;

[[group(1), binding(2)]]
var<storage, read> line_indices: LineIndices;

[[group(1), binding(3)]]
var<storage, read> line_meshes: LineMeshes;

[[group(1), binding(4)]]
var<storage, read> line_mesh_instances: LineMeshInstances;

[[group(1), binding(5)]]
var<storage, read> line_instances: LineInstances;

// Triangle vertex shader
[[stage(vertex)]]
fn vs_triangle(
    [[builtin(instance_index)]] instance: u32,
    in: TriangleVertexInput
) -> VertexOutput {
    let instance = triangle_mesh_instances.instances[instance];
    let instance_pos = instance.pos.xyz;
    let instance_rot = instance.rot;
    let instance_scale = instance.scale.xyz;

    let pos = instance_pos + (quat_mul(instance_rot, in.position) * instance_scale);

    var output: VertexOutput;
    output.position = r_uniforms.perspective * vec4<f32>(pos, 1.0);
    output.color = in.surface_color;
    output.intensity = in.intensity;
    output.delta_intensity = in.delta_intensity;
    return output;
}


// Line vertex shader
[[stage(vertex)]]
fn vs_line(
    [[builtin(instance_index)]] instance: u32,
    in: LineVertexInput
) -> VertexOutput {
    let line_instance = line_instances.instances[instance];
    let mesh_instance_id = line_instance.mesh_instance_id;
    let line_index = line_instance.line_index;

    let mesh_instance = line_mesh_instances.instances[mesh_instance_id];
    let instance_pos = mesh_instance.pos;
    let instance_rot = mesh_instance.rot;
    let instance_scale = mesh_instance.scale;
    let mesh_id = mesh_instance.mesh_id;

    let mesh = line_meshes.meshes[mesh_id];
    let vertex_offset = mesh.vertex_offset;
    let index_offset = mesh.index_offset;

    let idx0 = index_offset + line_index * u32(2);
    let idx1 = idx0 + u32(1);

    let i0 = vertex_offset + line_indices.indices[idx0];
    let i1 = vertex_offset + line_indices.indices[idx1];

    let v0 = mesh_vertices.vertices[i0];
    let v0_pos = instance_pos + (quat_mul(instance_rot, v0.m0.xyz) * instance_scale);
    let v0_surface_color = vec3<f32>(v0.m0.w, v0.m1.xy);
    let v0_line_color = vec3<f32>(v0.m1.zw, v0.m2.x);
    let v0_intensity = v0.m2.y;
    let v0_delta_intensity = v0.m2.z;

    let v1 = mesh_vertices.vertices[i1];
    let v1_pos = instance_pos + (quat_mul(instance_rot, v1.m0.xyz) * instance_scale);
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

// Fragment shader
[[stage(fragment)]]
fn fs_main(
    in: VertexOutput,
) -> FragmentOutput {
    var out: FragmentOutput;
    out.color = vec4<f32>(in.color * in.intensity, in.delta_intensity);
    return out;
}
