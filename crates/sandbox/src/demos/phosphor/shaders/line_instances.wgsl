struct MeshVertex {
    m0: vec4<f32>;
    m1: vec4<f32>;
    m2: vec4<f32>;
};

struct LineInstance {
    v0: MeshVertex;
    v1: MeshVertex;
};

struct MeshVertices {
    vertices: [[stride(48)]] array<MeshVertex>;
};

struct LineIndices {
    indices: [[stride(4)]] array<u32>;
};

struct LineInstances {
    instances: [[stride(96)]] array<LineInstance>;
};

[[group(0), binding(0)]] var<storage, read> mesh_vertices: MeshVertices;
[[group(0), binding(1)]] var<storage, read> line_indices: LineIndices;
[[group(0), binding(2)]] var<storage, read_write> line_instances: LineInstances;

[[stage(compute), workgroup_size(64)]]
fn main([[builtin(global_invocation_id)]] global_invocation_id: vec3<u32>) {
    let index = global_invocation_id.x;
    let total = arrayLength(&line_indices.indices) / u32(2);
    if (index >= total) {
        return;
    }

    let i0 = index * u32(2);
    let i1 = i0 + u32(1);

    let i0 = line_indices.indices[i0];
    let i1 = line_indices.indices[i1];

    var inst: LineInstance;
    inst.v0 = mesh_vertices.vertices[i0];
    inst.v1 = mesh_vertices.vertices[i1];
    line_instances.instances[index] = inst;
}
