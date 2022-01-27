use std::{borrow::Cow, sync::atomic::Ordering};

use antigen_core::{
    get_tagged_entity, Construct, PositionComponent, RotationComponent, ScaleComponent,
};
use antigen_wgpu::{
    buffer_size_of,
    wgpu::{BufferAddress, IndexFormat, LoadOp, Operations, COPY_BUFFER_ALIGNMENT},
    BufferDataBundle,
};
use hecs::{EntityBuilder, World};

use super::{
    BeamBuffer, BeamDepthBuffer, BeamTriangles, BeamMultisample, LineIndices, LineInstanceData,
    LineInstances, LineMeshData, LineMeshIdComponent, LineMeshIds, LineMeshIdsComponent,
    LineMeshInstanceData, LineMeshInstances, LineMeshes, Oscilloscope, PhosphorRenderer,
    StorageBuffers, TriangleIndices, TriangleMeshData, TriangleMeshIds, TriangleMeshIdsComponent,
    TriangleMeshInstanceData, TriangleMeshInstances, TriangleMeshes, Uniform, VertexData, Vertices,
    BLACK, BLUE, CLEAR_COLOR, GREEN, MAX_TRIANGLE_MESH_INSTANCES, RED, WHITE,
};

/// Pad a list of triangle indices to COPY_BUFFER_ALIGNMENT
pub fn pad_align_triangle_list(indices: &mut Vec<u16>) {
    while (buffer_size_of::<u16>() * indices.len() as BufferAddress) % COPY_BUFFER_ALIGNMENT > 0 {
        indices.extend(std::iter::repeat(0).take(3));
    }
}

/// Assemble mesh vertices
pub fn vertices_builder(world: &mut World, vertices: Vec<VertexData>) -> EntityBuilder {
    let mut builder = EntityBuilder::new();

    let vertex_entity = get_tagged_entity::<Vertices>(world).unwrap();

    let vertex_head = world
        .query_one_mut::<&mut antigen_wgpu::BufferLengthComponent>(vertex_entity)
        .unwrap();

    let vertex_count = vertices.len();

    let vertex_data = BufferDataBundle::new(
        vertices,
        buffer_size_of::<VertexData>()
            * vertex_head.fetch_add(vertex_count as BufferAddress, Ordering::Relaxed),
        vertex_entity,
    );
    builder.add_bundle(vertex_data);

    builder
}

/// Assemble line indices for pre-existing mesh vertices
pub fn line_indices_builder(world: &mut World, indices: Vec<u32>) -> EntityBuilder {
    let mut builder = EntityBuilder::new();

    let line_index_entity = get_tagged_entity::<LineIndices>(world).unwrap();
    let line_index_head = world
        .query_one_mut::<&mut antigen_wgpu::BufferLengthComponent>(line_index_entity)
        .unwrap();

    let index_count = indices.len();

    let index_data = BufferDataBundle::new(
        indices,
        buffer_size_of::<u32>()
            * line_index_head.fetch_add(index_count as BufferAddress, Ordering::Relaxed),
        line_index_entity,
    );

    builder.add_bundle(index_data);

    builder
}

/// Assembles mesh vertices and line indices
pub fn line_mesh_builder(
    world: &mut World,
    vertices: Vec<VertexData>,
    indices: Vec<u32>,
) -> EntityBuilder {
    let mut builder = EntityBuilder::new();

    let vertex_entity = get_tagged_entity::<Vertices>(world).unwrap();
    let line_index_entity = get_tagged_entity::<LineIndices>(world).unwrap();

    let vertex_offset = world
        .query_one_mut::<&antigen_wgpu::BufferLengthComponent>(vertex_entity)
        .unwrap()
        .load(Ordering::Relaxed);

    let index_offset = world
        .query_one_mut::<&antigen_wgpu::BufferLengthComponent>(line_index_entity)
        .unwrap()
        .load(Ordering::Relaxed);

    let vertex_count = vertices.len();
    let index_count = indices.len();

    builder.add_bundle(vertices_builder(world, vertices).build());

    builder.add_bundle(line_indices_builder(world, indices).build());

    builder.add_bundle(
        line_mesh_data_builder(
            world,
            vertex_offset as u32,
            vertex_count as u32,
            index_offset as u32,
            index_count as u32,
        )
        .build(),
    );

    builder
}

pub fn line_mesh_data_builder(
    world: &mut World,
    vertex_offset: u32,
    vertex_count: u32,
    index_offset: u32,
    index_count: u32,
) -> EntityBuilder {
    let mut builder = EntityBuilder::new();

    let line_mesh_entity = get_tagged_entity::<LineMeshes>(world).unwrap();
    let line_mesh_head = world
        .query_one_mut::<&mut antigen_wgpu::BufferLengthComponent>(line_mesh_entity)
        .unwrap();

    builder.add_bundle(BufferDataBundle::new(
        vec![LineMeshData {
            vertex_offset: vertex_offset,
            vertex_count: vertex_count,
            index_offset: index_offset,
            index_count: index_count,
        }],
        buffer_size_of::<LineMeshData>() * line_mesh_head.fetch_add(1, Ordering::Relaxed),
        line_mesh_entity,
    ));

    builder
}

pub fn line_mesh_instance_builder(
    world: &mut World,
    position: PositionComponent,
    rotation: RotationComponent,
    scale: ScaleComponent,
    mesh: &Cow<'static, str>,
) -> Option<EntityBuilder> {
    let mut builder = EntityBuilder::new();

    let query = world
        .query_mut::<&LineMeshIdsComponent>()
        .with::<LineMeshIds>();
    let (_, mesh_ids) = query.into_iter().next()?;
    let (line_mesh, line_count) = *mesh_ids.read().get(mesh)?;

    let line_mesh_instance_entity = get_tagged_entity::<LineMeshInstances>(world)?;
    let line_instance_entity = get_tagged_entity::<LineInstances>(world)?;

    let line_mesh_instance_head = world
        .query_one_mut::<&mut antigen_wgpu::BufferLengthComponent>(line_mesh_instance_entity)
        .ok()?;

    let mesh_instance = line_mesh_instance_head.fetch_add(1, Ordering::Relaxed);
    let base_offset = buffer_size_of::<LineMeshInstanceData>() * mesh_instance;

    builder.add_bundle(BufferDataBundle::new(
        position,
        base_offset,
        line_mesh_instance_entity,
    ));

    builder.add_bundle(BufferDataBundle::new(
        LineMeshIdComponent::construct(line_mesh),
        base_offset + buffer_size_of::<[f32; 3]>(),
        line_mesh_instance_entity,
    ));

    builder.add_bundle(BufferDataBundle::new(
        rotation,
        base_offset + buffer_size_of::<[f32; 4]>(),
        line_mesh_instance_entity,
    ));

    builder.add_bundle(BufferDataBundle::new(
        scale,
        base_offset + buffer_size_of::<[f32; 8]>(),
        line_mesh_instance_entity,
    ));

    let line_instance_head = world
        .query_one_mut::<&mut antigen_wgpu::BufferLengthComponent>(line_instance_entity)
        .ok()?;

    builder.add_bundle(BufferDataBundle::new(
        (0..line_count)
            .into_iter()
            .map(|i| LineInstanceData {
                mesh_instance: mesh_instance as u32,
                line_index: i,
            })
            .collect::<Vec<_>>(),
        buffer_size_of::<LineInstanceData>()
            * line_instance_head.fetch_add(line_count as BufferAddress, Ordering::Relaxed),
        line_instance_entity,
    ));

    Some(builder)
}

/// Assemble line indices for a vector of vertices in line list format
pub fn line_list_mesh_builder(world: &mut World, vertices: Vec<VertexData>) -> EntityBuilder {
    let mut vs = 0u32;
    let indices = vertices
        .chunks(2)
        .flat_map(|_| {
            let ret = [vs, vs + 1];
            vs += 2;
            ret
        })
        .collect::<Vec<_>>();

    line_mesh_builder(world, vertices, indices)
}

/// Assemble line indices for a vector of vertices in line strip format
pub fn line_strip_mesh_builder(world: &mut World, vertices: Vec<VertexData>) -> EntityBuilder {
    let mut indices = (0..vertices.len() as BufferAddress).collect::<Vec<_>>();

    let first = indices.remove(0) as u32;
    let last = indices.pop().unwrap() as u32;
    let inter = indices.into_iter().flat_map(|i| [i as u32, i as u32]);
    let indices = std::iter::once(first)
        .chain(inter)
        .chain(std::iter::once(last))
        .collect();

    line_mesh_builder(world, vertices, indices)
}

pub fn line_builder(
    world: &mut World,
    mesh: Cow<'static, str>,
    line_count: usize,
    color: (f32, f32, f32),
    intensity: f32,
    delta_intensity: f32,
) -> EntityBuilder {
    let mut builder = EntityBuilder::new();

    let line_mesh_entity = get_tagged_entity::<LineMeshes>(world).unwrap();

    let vertices = vec![
        VertexData {
            position: [0.0, 0.0, 0.0],
            surface_color: [color.0, color.1, color.2],
            line_color: [color.0, color.1, color.2],
            intensity,
            delta_intensity,
            ..Default::default()
        };
        line_count + 1
    ];

    let indices = (0..line_count as u32 + 1).collect::<Vec<_>>();
    let line_mesh = world
        .query_one_mut::<&mut antigen_wgpu::BufferLengthComponent>(line_mesh_entity)
        .unwrap()
        .load(Ordering::Relaxed) as u32;

    register_line_mesh_id(world, mesh, (line_mesh, line_count as u32));

    builder.add_bundle(line_mesh_builder(world, vertices, indices).build());

    builder
}

/// Assemble mesh vertices and indices
pub fn triangle_mesh_builder(
    world: &mut World,
    vertices: Vec<VertexData>,
    mut indices: Vec<u16>,
) -> EntityBuilder {
    let mut builder = EntityBuilder::new();

    let vertex_entity = get_tagged_entity::<Vertices>(world).unwrap();
    let triangle_index_entity = get_tagged_entity::<TriangleIndices>(world).unwrap();

    // Vertices
    let vertex_head = world
        .query_one_mut::<&mut antigen_wgpu::BufferLengthComponent>(vertex_entity)
        .unwrap();

    let vertex_count = vertices.len();
    let vertex_offset = buffer_size_of::<VertexData>()
        * vertex_head.fetch_add(vertex_count as u64, Ordering::Relaxed);

    builder.add_bundle(BufferDataBundle::new(
        vertices,
        vertex_offset,
        vertex_entity,
    ));

    // Indices
    pad_align_triangle_list(&mut indices);

    let triangle_index_head = world
        .query_one_mut::<&mut antigen_wgpu::BufferLengthComponent>(triangle_index_entity)
        .unwrap();

    let index_count = indices.len();
    let index_offset = buffer_size_of::<u16>()
        * triangle_index_head.fetch_add(index_count as u64, Ordering::Relaxed);

    builder.add_bundle(BufferDataBundle::new(
        indices,
        index_offset,
        triangle_index_entity,
    ));

    builder
}

pub fn triangle_mesh_data_builder(
    world: &mut World,
    vertex_count: u32,
    instance_count: u32,
    index_offset: u32,
    vertex_offset: u32,
) -> EntityBuilder {
    let mut builder = EntityBuilder::new();

    let triangle_mesh_entity = get_tagged_entity::<TriangleMeshes>(world).unwrap();
    let triangle_mesh_instance_entity = get_tagged_entity::<TriangleMeshInstances>(world).unwrap();

    let triangle_mesh_length = world
        .query_one_mut::<&mut antigen_wgpu::BufferLengthComponent>(triangle_mesh_entity)
        .unwrap();

    let triangle_mesh_head = triangle_mesh_length.fetch_add(1, Ordering::Relaxed);

    builder.add_bundle(BufferDataBundle::new(
        vec![TriangleMeshData {
            vertex_count,
            instance_count,
            index_offset,
            vertex_offset,
            ..Default::default()
        }],
        buffer_size_of::<TriangleMeshData>() * triangle_mesh_head,
        triangle_mesh_entity,
    ));

    let mut indexed_indirect_builder = triangle_indexed_indirect_builder(world, triangle_mesh_head);
    builder.add_bundle(indexed_indirect_builder.build());

    let triangle_mesh_instance_heads = world
        .query_one_mut::<&mut antigen_wgpu::BufferLengthsComponent>(triangle_mesh_instance_entity)
        .unwrap();

    triangle_mesh_instance_heads.write().push(0);

    builder
}

fn triangle_indexed_indirect_builder(world: &mut World, offset: u64) -> EntityBuilder {
    let mut builder = EntityBuilder::new();

    let beam_buffer_entity = get_tagged_entity::<BeamBuffer>(world).unwrap();
    let beam_multisample_entity = get_tagged_entity::<BeamMultisample>(world).unwrap();
    let beam_depth_buffer_entity = get_tagged_entity::<BeamDepthBuffer>(world).unwrap();
    let beam_mesh_pass_entity = get_tagged_entity::<BeamTriangles>(world).unwrap();
    let uniform_entity = get_tagged_entity::<Uniform>(world).unwrap();
    let storage_bind_group_entity = get_tagged_entity::<StorageBuffers>(world).unwrap();
    let renderer_entity = get_tagged_entity::<PhosphorRenderer>(world).unwrap();

    let vertex_entity = get_tagged_entity::<Vertices>(world).unwrap();
    let triangle_index_entity = get_tagged_entity::<TriangleIndices>(world).unwrap();
    let triangle_mesh_entity = get_tagged_entity::<TriangleMeshes>(world).unwrap();

    builder.add(BeamTriangles);

    builder.add_bundle(
        antigen_wgpu::RenderPassBundle::draw_indexed_indirect(
            1,
            Some("Beam Meshes".into()),
            vec![(
                beam_multisample_entity,
                Some(beam_buffer_entity),
                Operations {
                    load: LoadOp::Load,
                    store: true,
                },
            )],
            Some((
                beam_depth_buffer_entity,
                Some(Operations {
                    load: LoadOp::Load,
                    store: true,
                }),
                None,
            )),
            beam_mesh_pass_entity,
            vec![(vertex_entity, 0..480000)],
            Some((triangle_index_entity, 0..20000, IndexFormat::Uint16)),
            vec![
                (uniform_entity, vec![]),
                (
                    storage_bind_group_entity,
                    vec![
                        buffer_size_of::<TriangleMeshInstanceData>() as u32
                            * (MAX_TRIANGLE_MESH_INSTANCES * offset as usize) as u32,
                    ],
                ),
            ],
            vec![],
            None,
            None,
            None,
            None,
            (
                triangle_mesh_entity,
                buffer_size_of::<TriangleMeshData>() * offset,
            ),
            renderer_entity,
        )
        .build(),
    );

    builder
}

pub fn triangle_mesh_instance_builder(
    world: &mut World,
    mesh: &Cow<'static, str>,
    position: PositionComponent,
    rotation: RotationComponent,
    scale: ScaleComponent,
) -> Option<EntityBuilder> {
    let mut builder = EntityBuilder::new();

    let query = world
        .query_mut::<&TriangleMeshIdsComponent>()
        .with::<TriangleMeshIds>();
    let (_, mesh_ids) = query.into_iter().next()?;
    let triangle_mesh = *mesh_ids.read().get(mesh)?;

    let triangle_mesh_instance_entity = get_tagged_entity::<TriangleMeshInstances>(world)?;

    let triangle_mesh_instance_heads = world
        .query_one_mut::<&mut antigen_wgpu::BufferLengthsComponent>(triangle_mesh_instance_entity)
        .ok()?;

    let mut triangle_mesh_instance_head = triangle_mesh_instance_heads.write();
    let triangle_mesh_instance_head =
        triangle_mesh_instance_head.get_mut(triangle_mesh as usize)?;

    let base_offset = buffer_size_of::<TriangleMeshInstanceData>()
        * (*triangle_mesh_instance_head
            + (triangle_mesh * MAX_TRIANGLE_MESH_INSTANCES as u32) as BufferAddress);

    builder.add_bundle(BufferDataBundle::new(
        position,
        base_offset,
        triangle_mesh_instance_entity,
    ));

    builder.add_bundle(BufferDataBundle::new(
        rotation,
        base_offset + buffer_size_of::<[f32; 4]>(),
        triangle_mesh_instance_entity,
    ));

    builder.add_bundle(BufferDataBundle::new(
        scale,
        base_offset + buffer_size_of::<[f32; 8]>(),
        triangle_mesh_instance_entity,
    ));

    *triangle_mesh_instance_head += 1;

    Some(builder)
}

/// Assemble triangle indices for a list of vertices in triangle list format
pub fn triangle_list_mesh_builder(
    world: &mut World,
    mut base_index: u16,
    vertices: Vec<VertexData>,
) -> EntityBuilder {
    let indices = vertices
        .chunks(3)
        .flat_map(|_| {
            let is = [base_index, base_index + 1, base_index + 2];
            base_index += 3;
            is
        })
        .collect::<Vec<_>>();

    triangle_mesh_builder(world, vertices, indices)
}

/// Assemble triangle indices for a list of vertices in triangle fan format
pub fn triangle_fan_mesh_builder(
    world: &mut World,
    base_index: u16,
    vertices: Vec<VertexData>,
) -> EntityBuilder {
    let mut current_index = base_index;
    let indices = (0..vertices.len() - 2)
        .flat_map(|_| {
            let is = [base_index, current_index + 1, current_index + 2];
            current_index += 1;
            is
        })
        .collect::<Vec<_>>();

    triangle_mesh_builder(world, vertices, indices)
}

pub fn register_triangle_mesh_id(world: &mut World, key: Cow<'static, str>, triangle_mesh: u32) {
    let query = world
        .query_mut::<&mut TriangleMeshIdsComponent>()
        .with::<TriangleMeshIds>();
    let (_, mesh_ids) = query.into_iter().next().unwrap();
    mesh_ids.write().insert(key.into(), triangle_mesh);
}

pub fn register_line_mesh_id(world: &mut World, key: Cow<'static, str>, line_mesh: (u32, u32)) {
    let query = world
        .query_mut::<&mut LineMeshIdsComponent>()
        .with::<LineMeshIds>();
    let (_, mesh_ids) = query.into_iter().next().unwrap();
    mesh_ids.write().insert(key.into(), line_mesh);
}
