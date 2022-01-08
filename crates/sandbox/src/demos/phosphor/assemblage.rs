use antigen_core::Construct;
use antigen_wgpu::{
    buffer_size_of,
    wgpu::{BufferAddress, COPY_BUFFER_ALIGNMENT},
    BufferDataBundle,
};
use hecs::{Entity, EntityBuilder};

use super::{
    LineInstanceData, LineMeshData, LineMeshInstanceData, MeshVertexData, OriginComponent,
    Oscilloscope, BLACK, BLUE, GREEN, RED, WHITE,
};

/// Pad a list of triangle indices to COPY_BUFFER_ALIGNMENT
pub fn pad_align_triangle_list(indices: &mut Vec<u16>) {
    while (buffer_size_of::<u16>() * indices.len() as BufferAddress) % COPY_BUFFER_ALIGNMENT > 0 {
        indices.extend(std::iter::repeat(0).take(3));
    }
}

/// Assemble mesh vertices
pub enum MeshVerticesBundle {}

impl MeshVerticesBundle {
    pub fn builder(
        mesh_vertex_entity: Entity,
        vertex_head: &mut BufferAddress,
        vertices: Vec<MeshVertexData>,
    ) -> EntityBuilder {
        let mut builder = EntityBuilder::new();

        let vertex_count = vertices.len();

        let vertex_data = BufferDataBundle::new(
            vertices,
            buffer_size_of::<MeshVertexData>() * *vertex_head as BufferAddress,
            mesh_vertex_entity,
        );
        builder.add_bundle(vertex_data);

        *vertex_head += vertex_count as BufferAddress;

        builder
    }
}

/// Assemble line indices for pre-existing mesh vertices
pub enum LineIndicesBundle {}

impl LineIndicesBundle {
    pub fn builder(
        line_index_entity: Entity,
        line_index_head: &mut BufferAddress,
        indices: Vec<u32>,
    ) -> EntityBuilder {
        let mut builder = EntityBuilder::new();

        let index_count = indices.len();

        let index_data = BufferDataBundle::new(
            indices,
            buffer_size_of::<u32>() * *line_index_head as BufferAddress,
            line_index_entity,
        );

        builder.add_bundle(index_data);

        *line_index_head += index_count as BufferAddress;

        builder
    }
}

/// Assembles mesh vertices and line indices
pub enum LineMeshBundle {}

impl LineMeshBundle {
    pub fn builder(
        mesh_vertex_entity: Entity,
        line_index_entity: Entity,
        line_mesh_entity: Entity,
        vertex_head: &mut BufferAddress,
        index_head: &mut BufferAddress,
        line_mesh_head: &mut BufferAddress,
        vertices: Vec<MeshVertexData>,
        indices: Vec<u32>,
    ) -> EntityBuilder {
        let mut builder = EntityBuilder::new();

        let vertex_offset = *vertex_head;
        let vertex_count = vertices.len();
        let index_offset = *index_head;
        let index_count = indices.len();
        let line_mesh = *line_mesh_head;
        let line_count = index_count / 2;

        builder.add_bundle(
            MeshVerticesBundle::builder(mesh_vertex_entity, vertex_head, vertices).build(),
        );

        builder
            .add_bundle(LineIndicesBundle::builder(line_index_entity, index_head, indices).build());

        builder.add_bundle(
            LineMeshDataBundle::builder(
                line_mesh_entity,
                line_mesh_head,
                vertex_offset as u32,
                vertex_count as u32,
                index_offset as u32,
                index_count as u32,
            )
            .build(),
        );

        builder
    }
}

pub enum LineMeshDataBundle {}

impl LineMeshDataBundle {
    pub fn builder(
        line_mesh_entity: Entity,
        line_mesh_head: &mut BufferAddress,
        vertex_offset: u32,
        vertex_count: u32,
        index_offset: u32,
        index_count: u32,
    ) -> EntityBuilder {
        let mut builder = EntityBuilder::new();

        builder.add_bundle(BufferDataBundle::new(
            vec![LineMeshData {
                vertex_offset: vertex_offset,
                vertex_count: vertex_count,
                index_offset: index_offset,
                index_count: index_count,
            }],
            buffer_size_of::<LineMeshData>() * *line_mesh_head,
            line_mesh_entity,
        ));

        *line_mesh_head = *line_mesh_head + 1;

        builder
    }
}

pub enum LineMeshInstanceBundle {}

impl LineMeshInstanceBundle {
    pub fn builder(
        line_mesh_instance_entity: Entity,
        line_instance_entity: Entity,
        line_mesh_instance_head: &mut BufferAddress,
        line_instance_head: &mut BufferAddress,
        position: [f32; 3],
        line_mesh: u32,
        line_count: u32,
    ) -> EntityBuilder {
        let mut builder = EntityBuilder::new();

        builder.add_bundle(BufferDataBundle::new(
            vec![LineMeshInstanceData {
                position,
                mesh: line_mesh,
            }],
            buffer_size_of::<LineMeshInstanceData>() * *line_mesh_instance_head,
            line_mesh_instance_entity,
        ));

        builder.add_bundle(BufferDataBundle::new(
            (0..line_count)
                .into_iter()
                .map(|i| LineInstanceData {
                    mesh_instance: *line_mesh_instance_head as u32,
                    line_index: i,
                })
                .collect::<Vec<_>>(),
            buffer_size_of::<LineInstanceData>() * *line_instance_head,
            line_instance_entity,
        ));

        *line_mesh_instance_head = *line_mesh_instance_head + 1;
        *line_instance_head = *line_instance_head + line_count as BufferAddress;

        builder
    }
}

/// Assemble line indices for a vector of vertices in line list format
pub enum LineListBundle {}

impl LineListBundle {
    pub fn builder(
        mesh_vertex_entity: Entity,
        line_index_entity: Entity,
        line_mesh_entity: Entity,
        line_mesh_instance_entity: Entity,
        line_instance_entity: Entity,
        vertex_head: &mut BufferAddress,
        index_head: &mut BufferAddress,
        line_mesh_head: &mut BufferAddress,
        line_mesh_instance_head: &mut BufferAddress,
        line_instance_head: &mut BufferAddress,
        vertices: Vec<MeshVertexData>,
    ) -> EntityBuilder {
        let mut vs = 0u32;
        let indices = vertices
            .chunks(2)
            .flat_map(|_| {
                let ret = [vs, vs + 1];
                vs += 2;
                ret
            })
            .collect::<Vec<_>>();

        LineMeshBundle::builder(
            mesh_vertex_entity,
            line_index_entity,
            line_mesh_entity,
            vertex_head,
            index_head,
            line_mesh_head,
            vertices,
            indices,
        )
    }
}

/// Assemble line indices for a vector of vertices in line strip format
pub enum LineStripBundle {}

impl LineStripBundle {
    pub fn builder(
        mesh_vertex_entity: Entity,
        line_index_entity: Entity,
        line_mesh_entity: Entity,
        line_mesh_instance_entity: Entity,
        line_instance_entity: Entity,
        vertex_head: &mut BufferAddress,
        index_head: &mut BufferAddress,
        line_mesh_head: &mut BufferAddress,
        line_mesh_instance_head: &mut BufferAddress,
        line_instance_head: &mut BufferAddress,
        vertices: Vec<MeshVertexData>,
    ) -> EntityBuilder {
        let mut indices = (0..vertices.len() as BufferAddress).collect::<Vec<_>>();

        let first = indices.remove(0) as u32;
        let last = indices.pop().unwrap() as u32;
        let inter = indices.into_iter().flat_map(|i| [i as u32, i as u32]);
        let indices = std::iter::once(first)
            .chain(inter)
            .chain(std::iter::once(last))
            .collect();

        println!("Line strip indices: {:#?}", indices);

        LineMeshBundle::builder(
            mesh_vertex_entity,
            line_index_entity,
            line_mesh_entity,
            vertex_head,
            index_head,
            line_mesh_head,
            vertices,
            indices,
        )
    }
}

/// Assembles an oscilloscope entity
pub enum OscilloscopeBundle {}

impl OscilloscopeBundle {
    pub fn builder(
        mesh_vertex_entity: Entity,
        line_index_entity: Entity,
        line_mesh_entity: Entity,
        line_mesh_instance_entity: Entity,
        line_instance_entity: Entity,
        mesh_vertex_head: &mut BufferAddress,
        line_index_head: &mut BufferAddress,
        line_mesh_head: &mut BufferAddress,
        line_mesh_instance_head: &mut BufferAddress,
        line_instance_head: &mut BufferAddress,
        origin: (f32, f32, f32),
        color: (f32, f32, f32),
        oscilloscope: Oscilloscope,
        intensity: f32,
        delta_intensity: f32,
    ) -> EntityBuilder {
        let mut builder = EntityBuilder::new();

        builder.add(oscilloscope);

        let vertices = vec![
            MeshVertexData {
                position: [0.0, 0.0, 0.0],
                surface_color: [color.0, color.1, color.2],
                line_color: [color.0, color.1, color.2],
                intensity,
                delta_intensity,
                ..Default::default()
            },
            MeshVertexData {
                position: [0.0, 0.0, 0.0],
                surface_color: [color.0, color.1, color.2],
                line_color: [color.0, color.1, color.2],
                intensity,
                delta_intensity,
                ..Default::default()
            },
        ];

        let indices = vec![0u32, 1u32];
        let line_mesh = *line_mesh_head as u32;
        let line_count = 1;

        builder.add_bundle(
            LineMeshBundle::builder(
                mesh_vertex_entity,
                line_index_entity,
                line_mesh_entity,
                mesh_vertex_head,
                line_index_head,
                line_mesh_head,
                vertices,
                indices,
            )
            .build(),
        );

        builder.add_bundle(
            LineMeshInstanceBundle::builder(
                line_mesh_instance_entity,
                line_instance_entity,
                line_mesh_instance_head,
                line_instance_head,
                [origin.0, origin.1, origin.2],
                line_mesh,
                line_count
            )
            .build(),
        );

        builder
    }
}

/// Assemble mesh vertices and indices
pub enum TrianglesBundle {}

impl TrianglesBundle {
    pub fn builder(
        mesh_vertex_entity: Entity,
        mesh_index_entity: Entity,
        vertex_head: &mut BufferAddress,
        index_head: &mut BufferAddress,
        vertices: Vec<MeshVertexData>,
        mut indices: Vec<u16>,
    ) -> EntityBuilder {
        let mut builder = EntityBuilder::new();

        let vertex_offset = buffer_size_of::<MeshVertexData>() * *vertex_head;
        let index_offset = buffer_size_of::<u16>() * *index_head;

        pad_align_triangle_list(&mut indices);

        let vertex_count = vertices.len();
        let index_count = indices.len();

        builder.add_bundle(BufferDataBundle::new(
            vertices,
            vertex_offset,
            mesh_vertex_entity,
        ));

        builder.add_bundle(BufferDataBundle::new(
            indices,
            index_offset,
            mesh_index_entity,
        ));

        *vertex_head += vertex_count as BufferAddress;
        *index_head += index_count as BufferAddress;

        builder
    }
}

/// Assemble triangle indices for a list of vertices in triangle list format
pub enum TriangleListBundle {}

impl TriangleListBundle {
    pub fn builder(
        mesh_vertex_entity: Entity,
        mesh_index_entity: Entity,
        vertex_buffer_index: &mut BufferAddress,
        index_buffer_index: &mut BufferAddress,
        mut base_index: u16,
        vertices: Vec<MeshVertexData>,
    ) -> EntityBuilder {
        let indices = vertices
            .chunks(3)
            .flat_map(|_| {
                let is = [base_index, base_index + 1, base_index + 2];
                base_index += 3;
                is
            })
            .collect::<Vec<_>>();

        TrianglesBundle::builder(
            mesh_vertex_entity,
            mesh_index_entity,
            vertex_buffer_index,
            index_buffer_index,
            vertices,
            indices,
        )
    }
}

/// Assemble triangle indices for a list of vertices in triangle fan format
pub enum TriangleFanBundle {}

impl TriangleFanBundle {
    pub fn builder(
        mesh_vertex_entity: Entity,
        mesh_index_entity: Entity,
        vertex_buffer_index: &mut BufferAddress,
        index_buffer_index: &mut BufferAddress,
        base_index: u16,
        vertices: Vec<MeshVertexData>,
    ) -> EntityBuilder {
        let mut current_index = base_index;
        let indices = (0..vertices.len() - 2)
            .flat_map(|_| {
                let is = [base_index, current_index + 1, current_index + 2];
                current_index += 1;
                is
            })
            .collect::<Vec<_>>();

        TrianglesBundle::builder(
            mesh_vertex_entity,
            mesh_index_entity,
            vertex_buffer_index,
            index_buffer_index,
            vertices,
            indices,
        )
    }
}

/// Assemble the Box Bot
pub enum BoxBotBundle {}

impl BoxBotBundle {
    pub fn builders(
        mesh_vertex_entity: Entity,
        mesh_index_entity: Entity,
        line_index_entity: Entity,
        line_mesh_entity: Entity,
        line_mesh_instance_entity: Entity,
        line_instance_entity: Entity,
        mesh_vertex_head: &mut BufferAddress,
        mesh_index_head: &mut BufferAddress,
        line_index_head: &mut BufferAddress,
        line_mesh_head: &mut BufferAddress,
        line_mesh_instance_head: &mut BufferAddress,
        line_instance_head: &mut BufferAddress,
        (x, y, z): (f32, f32, f32),
    ) -> Vec<EntityBuilder> {
        let mut builders = vec![];

        // Cube lines
        builders.push(LineStripBundle::builder(
            mesh_vertex_entity,
            line_index_entity,
            line_mesh_entity,
            line_mesh_instance_entity,
            line_instance_entity,
            mesh_vertex_head,
            line_index_head,
            line_mesh_head,
            line_mesh_instance_head,
            line_instance_head,
            vec![
                MeshVertexData::new((-25.0, -25.0, -25.0), RED, RED, 2.0, -30.0),
                MeshVertexData::new((25.0, -25.0, -25.0), GREEN, GREEN, 2.0, -30.0),
                MeshVertexData::new((25.0, -25.0, 25.0), BLUE, GREEN, 2.0, -30.0),
                MeshVertexData::new((-25.0, -25.0, 25.0), WHITE, WHITE, 2.0, -30.0),
                MeshVertexData::new((-25.0, -25.0, -25.0), RED, RED, 2.0, -30.0),
            ]
            .into_iter()
            .map(|mut v| {
                v.position[0] += x;
                v.position[1] += y;
                v.position[2] += z;
                v
            })
            .collect(),
        ));

        builders.push(LineStripBundle::builder(
            mesh_vertex_entity,
            line_index_entity,
            line_mesh_entity,
            line_mesh_instance_entity,
            line_instance_entity,
            mesh_vertex_head,
            line_index_head,
            line_mesh_head,
            line_mesh_instance_head,
            line_instance_head,
            vec![
                MeshVertexData::new((-25.0, 25.0, -25.0), RED, RED, 2.0, -30.0),
                MeshVertexData::new((25.0, 25.0, -25.0), GREEN, RED, 2.0, -30.0),
                MeshVertexData::new((25.0, 25.0, 25.0), BLUE, RED, 2.0, -30.0),
                MeshVertexData::new((-25.0, 25.0, 25.0), WHITE, RED, 2.0, -30.0),
                MeshVertexData::new((-25.0, 25.0, -25.0), BLACK, RED, 2.0, -30.0),
            ]
            .into_iter()
            .map(|mut v| {
                v.position[0] += x;
                v.position[1] += y;
                v.position[2] += z;
                v
            })
            .collect(),
        ));

        builders.push(LineListBundle::builder(
            mesh_vertex_entity,
            line_index_entity,
            line_mesh_entity,
            line_mesh_instance_entity,
            line_instance_entity,
            mesh_vertex_head,
            line_index_head,
            line_mesh_head,
            line_mesh_instance_head,
            line_instance_head,
            vec![
                MeshVertexData::new((-25.0, -25.0, -25.0), RED, RED, 2.0, -30.0),
                MeshVertexData::new((-25.0, 25.0, -25.0), RED, RED, 2.0, -30.0),
                MeshVertexData::new((25.0, -25.0, -25.0), GREEN, GREEN, 2.0, -30.0),
                MeshVertexData::new((25.0, 25.0, -25.0), GREEN, GREEN, 2.0, -30.0),
                MeshVertexData::new((25.0, -25.0, 25.0), BLUE, BLUE, 2.0, -30.0),
                MeshVertexData::new((25.0, 25.0, 25.0), BLUE, BLUE, 2.0, -30.0),
                MeshVertexData::new((-25.0, -25.0, 25.0), WHITE, WHITE, 2.0, -30.0),
                MeshVertexData::new((-25.0, 25.0, 25.0), WHITE, WHITE, 2.0, -30.0),
            ]
            .into_iter()
            .map(|mut v| {
                v.position[0] += x;
                v.position[1] += y;
                v.position[2] += z;
                v
            })
            .collect(),
        ));

        // Body cube
        builders.push(TrianglesBundle::builder(
            mesh_vertex_entity,
            mesh_index_entity,
            mesh_vertex_head,
            mesh_index_head,
            vec![
                MeshVertexData::new((1.0, 1.0, 1.0), BLACK, BLACK, 0.0, -16.0),
                MeshVertexData::new((-1.0, 1.0, 1.0), BLACK, BLACK, 0.0, -16.0),
                MeshVertexData::new((-1.0, 1.0, -1.0), BLACK, BLACK, 0.0, -16.0),
                MeshVertexData::new((1.0, 1.0, -1.0), BLACK, BLACK, 0.0, -16.0),
                MeshVertexData::new((1.0, -1.0, 1.0), BLACK, BLACK, 0.0, -16.0),
                MeshVertexData::new((-1.0, -1.0, 1.0), BLACK, BLACK, 0.0, -16.0),
                MeshVertexData::new((-1.0, -1.0, -1.0), BLACK, BLACK, 0.0, -16.0),
                MeshVertexData::new((1.0, -1.0, -1.0), BLACK, BLACK, 0.0, -16.0),
            ]
            .into_iter()
            .map(|mut vd| {
                vd.position[0] *= 25.0;
                vd.position[1] *= 25.0;
                vd.position[2] *= 25.0;
                vd.position[0] += x;
                vd.position[1] += y;
                vd.position[2] += z;
                vd
            })
            .collect(),
            vec![
                // Top
                0, 1, 2, 0, 2, 3, // Bottom
                4, 7, 5, 7, 6, 5, // Front
                3, 2, 6, 3, 6, 7, // Back
                0, 5, 1, 0, 4, 5, // Right
                0, 3, 7, 0, 7, 4, // Left
                1, 5, 6, 1, 6, 2,
            ]
            .into_iter()
            .map(|id| id + (*mesh_vertex_head) as u16)
            .collect(),
        ));

        // Visor cube
        builders.push(TrianglesBundle::builder(
            mesh_vertex_entity,
            mesh_index_entity,
            mesh_vertex_head,
            mesh_index_head,
            vec![
                MeshVertexData::new((1.0, 1.0, 1.0), RED, RED, 2.0, -14.0),
                MeshVertexData::new((-1.0, 1.0, 1.0), RED, RED, 2.0, -14.0),
                MeshVertexData::new((-1.0, 1.0, -1.0), RED, RED, 2.0, -14.0),
                MeshVertexData::new((1.0, 1.0, -1.0), RED, RED, 2.0, -14.0),
                MeshVertexData::new((1.0, -1.0, 1.0), RED, RED, 2.0, -14.0),
                MeshVertexData::new((-1.0, -1.0, 1.0), RED, RED, 2.0, -14.0),
                MeshVertexData::new((-1.0, -1.0, -1.0), RED, RED, 2.0, -14.0),
                MeshVertexData::new((1.0, -1.0, -1.0), RED, RED, 2.0, -14.0),
            ]
            .into_iter()
            .map(|mut vd| {
                vd.position[0] *= 10.0;
                vd.position[1] *= 2.5;
                vd.position[2] *= 2.5;
                vd.position[2] -= 25.0;
                vd.position[0] += x;
                vd.position[1] += y;
                vd.position[2] += z;
                vd
            })
            .collect(),
            vec![
                // Top
                0, 1, 2, 0, 2, 3, // Bottom
                4, 7, 5, 7, 6, 5, // Front
                3, 2, 6, 3, 6, 7, // Back
                0, 5, 1, 0, 4, 5, // Right
                0, 3, 7, 0, 7, 4, // Left
                1, 5, 6, 1, 6, 2,
            ]
            .into_iter()
            .map(|id| id + (*mesh_vertex_head as u16))
            .collect(),
        ));

        builders
    }
}

/*
pub fn assemble_png_texture_with_usage<C, U, I>(
    cmd: &mut legion::systems::CommandBuffer,
    renderer_entity: Entity,
    label: Option<&'static str>,
    png_bytes: &[u8],
) where
    C: Construct<Vec<u8>, I> + Send + Sync + 'static,
    U: Send + Sync + 'static,
{
    // Gradients texture
    let decoder = png::Decoder::new(std::io::Cursor::new(png_bytes));
    let mut reader = decoder.read_info().unwrap();
    let mut buf = vec![0; reader.output_buffer_size()];
    let info = reader.next_frame(&mut buf).unwrap();

    let size = Extent3d {
        width: info.width,
        height: info.height,
        depth_or_array_layers: 1,
    };

    cmd.assemble_wgpu_texture_with_usage::<U>(
        renderer_entity,
        TextureDescriptor {
            label,
            size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: TextureDimension::D2,
            format: TextureFormat::Rgba8UnormSrgb,
            usage: TextureUsages::TEXTURE_BINDING | TextureUsages::COPY_DST,
        },
    );

    cmd.assemble_wgpu_texture_data_with_usage::<U, _>(
        renderer_entity,
        C::construct(buf),
        ImageCopyTextureBase {
            texture: (),
            mip_level: 0,
            origin: Default::default(),
            aspect: TextureAspect::All,
        },
        ImageDataLayout {
            offset: 0,
            bytes_per_row: Some(NonZeroU32::new(info.line_size as u32).unwrap()),
            rows_per_image: Some(NonZeroU32::new(size.height).unwrap()),
        },
    );

    cmd.assemble_wgpu_texture_view_with_usage::<U>(
        renderer_entity,
        renderer_entity,
        TextureViewDescriptor {
            label,
            format: None,
            dimension: None,
            aspect: TextureAspect::All,
            base_mip_level: 0,
            mip_level_count: None,
            base_array_layer: 0,
            array_layer_count: None,
        },
    );
}
*/
