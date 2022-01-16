//           * Will prevent per-pixel animations,
//             but allow for proper color combination in phosphor buffer
//           * Fixes trails all fading to red, as the background is cleared to red
//
// TODO: [✓] Implement MSAA for beam buffer
//           * Will likely involve moving gradient evaluation into the beam vertex shaders
//
// TODO: [✓] Remove gradient code
//
// TODO: 3D Rendering
//       * Need to figure out how to make a phosphor decay model work with a rotateable camera
//       * Want to avoid UE4-style whole screen smearing
//       [✓] Depth offset for beam lines
//           * Ideally should appear as if they're 3D w.r.t. depth vs meshes
//             * No Z-fighting with parent geometry
//           * cos(length(vertex_pos.xy))?
//           * Doesn't achieve desired effect
//             * Render pipeline depth bias not controllable enough
//             * Fragment shader math too complex, needs depth sample
//             * Mesh inset doesn't align with line geometry
//             * Applying in vertex space w/appropriate projection matrix tweaks is viable
//
//       [✓] Compute shader for generating lines from mesh vertex buffer and line index buffer
//
//       [✗] Sort lines back-to-front for proper overlay behavior
//           * Not necessary with proper additive rendering
//
//       [✓] Combine duplicate lines via averaging
//           * Will prevent Z-fighting
//           [✓] Initial implementation
//           [✓] Correct evaluation when va0 == vb1 && vb0 == va1
//
//       [>] Render triangle meshes from map file
//           * Can use to clear a specific area to black w/a given decay rate
//           [✓] Basic implementation
//           [✓] More robust predicate for face pruning
//           [✓] Fix erroneous line indices in map geometry
//               * Lines appear to be using mesh vertices rather than line vertices
//                 * Suggested by the purple color
//               * Not dependent on the presence of other geometry
//               * This would suggest an issue in assemble_map_geometry
//           [✗] Apply interior face filter recursively to prune leftover faces
//               * Doesn't work for closed loops like pillars
//               * Not worth the additional cost to remove caps
//               * May be worth looking at some means to detect and prune caps
//           [✓] Fix index buffer alignment crash with test map
//           [✓] Allow lines to override vertex color
//               * Allows for black geo with colored lines without duplicating verts
//           [ ] Account for portal entities when calculating internal faces
//               * Will need some predicate that can be passed to the InternalFaces constructor
//           [ ] Investigate calculating subsectors from internal faces
//           [✓] Paralellize shambler
//
//       [✓] Figure out how to flush command buffers at runtime
//           * Needed to add, remove components or entities
//           * Want to avoid the Godot issue of stalling the main thread for object allocation
//           * Only the allocating thread should block
//           * This would suggest maintaining one world per thread and
//             shuttling data between them via channel through a centralized 'world manager'
//           * May be wiser to downgrade the RwLock-first approach back to special-case usage
//           * Is there a way to compose systems that doesn't involve customized legion types?
//
//       [ ] Changed<PathComponent> map file reloading
//           * Will allow a system to read ArgsComponent and load a map based on its value
//
//       [ ] Investigate infinite perspective projection + reversed Z
//
// TODO: [ ] Implement HDR bloom
//           * Render mipmaps for final buffer
//           * Render HDR bloom using mipmaps
//
//       [ ] Implement automatic line smearing via compute shader
//           * Double-buffer vertices, use to construct quads
//           * Will need to update backbuffer if lines are added / removed at runtime
//             * Ex. Via frustum culling or portal rendering
//
//       [ ] Is automatic mesh smearing viable?
//
//       [ ] Experiment with scrolling / smearing the phosphor buffer based on camera motion
//           * Should be able to move it in a somewhat perspective-correct fashion
//
//       [ ] Sort meshes front-to-back for optimal z-fail behavior
//
//       [ ] Downsample prototype.wad textures to 1x1px to determine color
//
// TODO: [ ] Implement LUT mapping via 3D texture
//           * Replaces per-fragment gradient animation
//           * Will need to figure out how to generate data
//             * Rendering to 3D texture
//             * Unit LUT is just a color cube with B/RGB/CMY/W vertices
//
//       * MechWarrior 2 gradient skybox background
//         * Setting for underlay / overlay behavior
//         * Overlay acts like a vectrex color overlay
//         * Underlay respects depth and doesn't draw behind solid objects
//

mod assemblage;
mod components;
mod render_passes;
mod svg_lines;
mod systems;

use antigen_fs::{load_file_string, FilePathComponent};
pub use assemblage::*;
pub use components::*;
pub use render_passes::*;
pub use svg_lines::*;
pub use systems::*;

use expression::{EvalTrait, Expression};
use std::{
    collections::BTreeMap, error::Error, path::PathBuf, sync::atomic::Ordering, time::Instant,
};

use antigen_winit::{
    winit::{
        event::{Event, WindowEvent},
        event_loop::{ControlFlow, EventLoopWindowTarget},
    },
    EventLoopHandler, RedrawUnconditionally, WindowComponent,
};

use antigen_core::{
    send_clone_query, send_component, Construct, Indirect, Lift, MessageContext, MessageResult,
    SendTo, TaggedEntitiesComponent, WorldChannel,
};

use antigen_wgpu::{
    buffer_size_of, spawn_shader_from_file_string,
    wgpu::{
        AddressMode, BufferAddress, BufferDescriptor, BufferUsages, Color,
        CommandEncoderDescriptor, Extent3d, FilterMode, IndexFormat, LoadOp, Maintain, Operations,
        SamplerDescriptor, TextureAspect, TextureDescriptor, TextureDimension, TextureFormat,
        TextureUsages, TextureViewDescriptor,
    },
    AdapterComponent, BindGroupComponent, BindGroupLayoutComponent, BufferComponent,
    BufferLengthComponent, BufferLengthsComponent, DeviceComponent, InstanceComponent,
    QueueComponent, RenderPipelineComponent, ShaderModuleComponent,
    ShaderModuleDescriptorComponent, SurfaceConfigurationComponent, TextureViewComponent,
};

use antigen_shambler::shambler::{
    brush::BrushId,
    entity::EntityId,
    face::FaceId,
    shalrath::repr::{Properties, Property},
    GeoMap,
};

use hecs::{Entity, EntityBuilder, World};

use crate::{Filesystem, Game, Render};

const HDR_TEXTURE_FORMAT: TextureFormat = TextureFormat::Rgba16Float;
const MAX_MESH_VERTICES: usize = 10000;
const MAX_TRIANGLE_INDICES: usize = 10000;
const MAX_TRIANGLE_MESHES: usize = 100;
const MAX_TRIANGLE_MESH_INSTANCES: usize = 256;
const MAX_LINE_INDICES: usize = 20000;
const MAX_LINE_MESHES: usize = 100;
const MAX_LINE_MESH_INSTANCES: usize = 400;
const MAX_LINE_INSTANCES: usize = MAX_LINE_INDICES / 2;
const CLEAR_COLOR: antigen_wgpu::wgpu::Color = antigen_wgpu::wgpu::Color {
    r: 0.0,
    g: 0.0,
    b: 0.0,
    a: -200.0,
};

pub const BLACK: (f32, f32, f32) = (0.0, 0.0, 0.0);
pub const RED: (f32, f32, f32) = (1.0, 0.0, 0.0);
pub const GREEN: (f32, f32, f32) = (0.0, 1.0, 0.0);
pub const BLUE: (f32, f32, f32) = (0.0, 0.0, 1.0);
pub const WHITE: (f32, f32, f32) = (1.0, 1.0, 1.0);

pub fn orthographic_matrix(aspect: f32, zoom: f32, near: f32, far: f32) -> [[f32; 4]; 4] {
    let projection = nalgebra_glm::ortho_lh_zo(
        -zoom * aspect,
        zoom * aspect,
        -zoom,
        zoom,
        0.0,
        zoom * (far - near) * 2.0,
    );
    projection.into()
}

pub fn perspective_matrix(
    aspect: f32,
    (ofs_x, ofs_y): (f32, f32),
    near: f32,
    far: f32,
) -> [[f32; 4]; 4] {
    let x = ofs_x * std::f32::consts::PI;
    let view = nalgebra_glm::look_at_lh(
        &nalgebra::vector![x.sin() * 300.0, ofs_y * 150.0, -x.cos() * 300.0],
        &nalgebra::vector![0.0, 0.0, 0.0],
        &nalgebra::Vector3::y_axis(),
    );
    let projection = nalgebra_glm::perspective_lh_zo(aspect, (45.0f32).to_radians(), near, far);

    let matrix = projection * view;

    matrix.into()
}

fn circle_strip(subdiv: usize) -> Vec<LineVertexData> {
    let subdiv = subdiv as isize;
    let half = 1 + subdiv;

    // Generate left quarter-circle
    let mut left = (-half..1)
        .map(|i| i as f32 / half as f32)
        .map(|f| {
            let f = f * (std::f32::consts::PI * 0.5);
            (f.sin(), f.cos(), 0.0)
        })
        .collect::<Vec<_>>();

    // Generate right quarter-circle
    let mut right = (0..half + 1)
        .map(|i| i as f32 / half as f32)
        .map(|f| {
            let f = f * (std::f32::consts::PI * 0.5);
            (f.sin(), f.cos(), 1.0)
        })
        .collect::<Vec<_>>();

    // Find intermediate vertices and duplicate them with negative Y
    let first = left.remove(0);
    let last = right.pop().unwrap();

    let inter = left
        .into_iter()
        .chain(right.into_iter())
        .flat_map(|(x, y, s)| [(x, -y, s), (x, y, s)]);

    // Stitch the first, intermediate and last vertices back together and convert into line vertex data
    std::iter::once(first)
        .chain(inter)
        .chain(std::iter::once(last))
        .map(|(x, y, s)| LineVertexData {
            position: [x, y, -1.0],
            end: s,
            ..Default::default()
        })
        .collect()
}

fn load_shader_message<P: Copy + Into<PathBuf>>(
    shader_path: P,
    entity: Entity,
) -> impl for<'a, 'b> FnOnce(MessageContext<'a, 'b>) -> MessageResult<'a, 'b> {
    move |ctx| {
        ctx.lift()
            .and_then(load_file_string(shader_path))
            .and_then(spawn_shader_from_file_string(shader_path))
            .and_then(
                send_component::<ShaderModuleDescriptorComponent, Render, _>(
                    FilePathComponent::construct(shader_path.into()),
                    entity,
                ),
            )
            .and_then(send_component::<ShaderModuleComponent, Render, _>(
                FilePathComponent::construct(shader_path.into()),
                entity,
            ))
    }
}

fn load_shader<T: Send + Sync + 'static, P: Copy + Into<PathBuf> + Send + Sync + 'static>(
    channel: &WorldChannel,
    entity: Entity,
    shader_path: P,
) {
    channel
        .send_to::<T>(load_shader_message(shader_path, entity))
        .unwrap();
}

fn load_map_message<U: Send + Sync + 'static, P: Copy + Into<PathBuf>>(
    map_path: P,
    entity: Entity,
) -> impl for<'a, 'b> FnOnce(MessageContext<'a, 'b>) -> MessageResult<'a, 'b> {
    move |ctx| {
        ctx.lift()
            .and_then(load_file_string(map_path))
            .and_then(antigen_shambler::parse_map_file_string(map_path))
    }
}

fn load_map<
    U: Send + Sync + 'static,
    T: Send + Sync + 'static,
    P: Copy + Into<PathBuf> + Send + Sync + 'static,
>(
    channel: &WorldChannel,
    entity: Entity,
    map_path: P,
) {
    channel
        .send_to::<T>(load_map_message::<U, _>(map_path, entity))
        .unwrap();
}

fn insert_tagged_entity<Q: hecs::Query + Send + Sync + 'static, T: 'static>(
) -> impl for<'a, 'b> Fn(MessageContext<'a, 'b>) -> Result<MessageContext<'a, 'b>, Box<dyn Error>> {
    move |mut ctx: MessageContext| {
        let (world, _) = &mut ctx;

        let (entity, _) = world.query_mut::<Q>().into_iter().next().unwrap();

        let (_, named_entities) = if let Some(component) = world
            .query_mut::<&mut TaggedEntitiesComponent>()
            .into_iter()
            .next()
        {
            component
        } else {
            let entity = world.spawn((TaggedEntitiesComponent::default(),));
            (
                entity,
                world
                    .query_one_mut::<&mut TaggedEntitiesComponent>(entity)
                    .unwrap(),
            )
        };

        let tag_id = std::any::TypeId::of::<T>();
        named_entities.insert(tag_id, entity);
        println!(
            "Thread {:?} Inserted name {:?} for entity {:?}",
            std::thread::current().name().unwrap(),
            tag_id,
            entity
        );

        Ok(ctx)
    }
}

pub fn assemble(world: &mut World, channel: &WorldChannel) {
    let window_entity = world.reserve_entity();
    let renderer_entity = world.reserve_entity();

    let (wgpu_entity, _) = world
        .query_mut::<(
            &InstanceComponent,
            &AdapterComponent,
            &DeviceComponent,
            &QueueComponent,
        )>()
        .into_iter()
        .next()
        .unwrap();

    send_clone_query::<
        (
            &InstanceComponent,
            &AdapterComponent,
            &DeviceComponent,
            &QueueComponent,
        ),
        Game,
    >(wgpu_entity)((world, channel))
    .unwrap();

    // Uniforms
    let mut builder = EntityBuilder::new();
    let bundle = builder
        .add(Uniform)
        .add(BindGroupLayoutComponent::default())
        .add(BindGroupComponent::default())
        .add_bundle(antigen_wgpu::BufferBundle::new(BufferDescriptor {
            label: Some("Uniform Buffer"),
            size: buffer_size_of::<UniformData>(),
            usage: BufferUsages::UNIFORM | BufferUsages::INDIRECT | BufferUsages::COPY_DST,
            mapped_at_creation: false,
        }))
        .build();
    let uniform_entity = world.spawn(bundle);

    // Vertices
    let mut builder = EntityBuilder::new();
    let bundle = builder
        .add(Vertices)
        .add_bundle(antigen_wgpu::BufferBundle::new(BufferDescriptor {
            label: Some("Vertex Buffer"),
            size: buffer_size_of::<VertexData>() * MAX_MESH_VERTICES as BufferAddress,
            usage: BufferUsages::VERTEX | BufferUsages::STORAGE | BufferUsages::COPY_DST,
            mapped_at_creation: false,
        }))
        .add(BufferLengthComponent::default())
        .build();
    let vertex_entity = world.spawn(bundle);
    insert_tagged_entity::<(&Vertices, &BufferComponent), Vertices>()((world, channel)).unwrap();

    // Mesh IDs
    let mut builder = EntityBuilder::new();
    builder.add(MeshIds);
    builder.add(MeshIdsComponent::default());
    let mesh_ids_entity = world.spawn(builder.build());

    // Triangle Indices
    let mut builder = EntityBuilder::new();
    let bundle = builder
        .add(TriangleIndices)
        .add_bundle(antigen_wgpu::BufferBundle::new(BufferDescriptor {
            label: Some("Triangle Index Buffer"),
            size: buffer_size_of::<TriangleIndexData>() * MAX_TRIANGLE_INDICES as BufferAddress,
            usage: BufferUsages::INDEX | BufferUsages::COPY_DST,
            mapped_at_creation: false,
        }))
        .add(BufferLengthComponent::default())
        .build();
    let triangle_index_entity = world.spawn(bundle);
    insert_tagged_entity::<(&TriangleIndices, &BufferComponent), TriangleIndices>()((
        world, channel,
    ))
    .unwrap();

    // Triangle Meshes
    let mut builder = EntityBuilder::new();
    let bundle = builder
        .add(TriangleMeshes)
        .add_bundle(antigen_wgpu::BufferBundle::new(BufferDescriptor {
            label: Some("Triangle Mesh Buffer"),
            size: buffer_size_of::<TriangleMeshData>() * MAX_TRIANGLE_MESHES as BufferAddress,
            usage: BufferUsages::INDIRECT | BufferUsages::COPY_DST,
            mapped_at_creation: false,
        }))
        .add(BufferLengthComponent::default())
        .build();
    let triangle_mesh_entity = world.spawn(bundle);
    insert_tagged_entity::<(&TriangleMeshes, &BufferComponent), TriangleMeshes>()((world, channel))
        .unwrap();

    // Triangle Mesh Instances
    let mut builder = EntityBuilder::new();
    let bundle = builder
        .add(TriangleMeshInstances)
        .add_bundle(antigen_wgpu::BufferBundle::new(BufferDescriptor {
            label: Some("Triangle Mesh Instance Buffer"),
            size: buffer_size_of::<TriangleMeshInstanceData>()
                * (MAX_TRIANGLE_MESHES * MAX_TRIANGLE_MESH_INSTANCES) as BufferAddress,
            usage: BufferUsages::STORAGE | BufferUsages::COPY_DST,
            mapped_at_creation: false,
        }))
        .add(BufferLengthsComponent::default())
        .build();

    let triangle_mesh_instance_entity = world.spawn(bundle);

    insert_tagged_entity::<(&TriangleMeshInstances, &BufferComponent), TriangleMeshInstances>()((
        world, channel,
    ))
    .unwrap();

    // Line Vertices
    let vertices = circle_strip(2);
    let mut builder = EntityBuilder::new();
    let line_vertex_entity = world.reserve_entity();
    let bundle = builder
        .add(LineVertices)
        .add_bundle(antigen_wgpu::BufferBundle::new(BufferDescriptor {
            label: Some("Line Vertex Buffer"),
            size: buffer_size_of::<LineVertexData>() * vertices.len() as BufferAddress,
            usage: BufferUsages::VERTEX | BufferUsages::COPY_DST,
            mapped_at_creation: false,
        }))
        .add_bundle(antigen_wgpu::BufferDataBundle::new(
            vertices,
            0,
            line_vertex_entity,
        ))
        .build();
    world.insert(line_vertex_entity, bundle).unwrap();

    // Line Indices
    let mut builder = EntityBuilder::new();
    let bundle = builder
        .add(LineIndices)
        .add_bundle(antigen_wgpu::BufferBundle::new(BufferDescriptor {
            label: Some("Line Index Buffer"),
            size: buffer_size_of::<LineIndexData>() * MAX_LINE_INDICES as BufferAddress,
            usage: BufferUsages::STORAGE | BufferUsages::COPY_DST,
            mapped_at_creation: false,
        }))
        .add(BufferLengthComponent::default())
        .build();
    let line_index_entity = world.spawn(bundle);
    insert_tagged_entity::<(&LineIndices, &BufferComponent), LineIndices>()((world, channel))
        .unwrap();

    // Line Meshes
    let mut builder = EntityBuilder::new();
    let bundle = builder
        .add(LineMeshes)
        .add_bundle(antigen_wgpu::BufferBundle::new(BufferDescriptor {
            label: Some("Mesh Buffer"),
            size: buffer_size_of::<LineMeshData>() * MAX_LINE_MESHES as BufferAddress,
            usage: BufferUsages::STORAGE | BufferUsages::COPY_DST,
            mapped_at_creation: false,
        }))
        .add(BufferLengthComponent::default())
        .build();
    let line_mesh_entity = world.spawn(bundle);
    insert_tagged_entity::<(&LineMeshes, &BufferComponent), LineMeshes>()((world, channel))
        .unwrap();

    // Line Mesh Instances
    let mut builder = EntityBuilder::new();
    let bundle = builder
        .add(LineMeshInstances)
        .add_bundle(antigen_wgpu::BufferBundle::new(BufferDescriptor {
            label: Some("Line Mesh Instance Buffer"),
            size: buffer_size_of::<LineMeshInstanceData>()
                * MAX_LINE_MESH_INSTANCES as BufferAddress,
            usage: BufferUsages::STORAGE | BufferUsages::COPY_DST,
            mapped_at_creation: false,
        }))
        .add(BufferLengthComponent::default())
        .build();
    let line_mesh_instance_entity = world.spawn(bundle);

    insert_tagged_entity::<(&LineMeshInstances, &BufferComponent), LineMeshInstances>()((
        world, channel,
    ))
    .unwrap();

    // Line Instances
    let mut builder = EntityBuilder::new();
    let bundle = builder
        .add(LineInstances)
        .add_bundle(antigen_wgpu::BufferBundle::new(BufferDescriptor {
            label: Some("Line Instance Buffer"),
            size: buffer_size_of::<LineInstanceData>() * MAX_LINE_INSTANCES as BufferAddress,
            usage: BufferUsages::VERTEX | BufferUsages::STORAGE | BufferUsages::COPY_DST,
            mapped_at_creation: false,
        }))
        .add(BufferLengthComponent::default())
        .build();
    let line_instance_entity = world.spawn(bundle);

    insert_tagged_entity::<(&LineInstances, &BufferComponent), LineInstances>()((world, channel))
        .unwrap();

    // Clone buffers to game thread
    send_clone_query::<
        (
            &TriangleMeshInstances,
            &BufferComponent,
            &BufferLengthsComponent,
        ),
        Game,
    >(triangle_mesh_instance_entity)((world, channel))
    .unwrap();


    // Total time entity
    let mut builder = EntityBuilder::new();
    let bundle = builder
        .add(StartTimeComponent::construct(Instant::now()))
        .add_bundle(antigen_wgpu::BufferDataBundle::new(
            TotalTimeComponent::construct(0.0),
            buffer_size_of::<[[f32; 4]; 4]>() * 2,
            uniform_entity,
        ))
        .build();
    let _total_time_entity = world.spawn(bundle);

    // Delta time entity
    let mut builder = EntityBuilder::new();
    let bundle = builder
        .add(TimestampComponent::construct(Instant::now()))
        .add_bundle(antigen_wgpu::BufferDataBundle::new(
            DeltaTimeComponent::construct(1.0 / 60.0),
            (buffer_size_of::<[[f32; 4]; 4]>() * 2) + buffer_size_of::<f32>(),
            uniform_entity,
        ))
        .build();
    let _delta_time_entity = world.spawn(bundle);

    // Perspective matrix entity
    let mut builder = EntityBuilder::new();
    let bundle = builder
        .add(Perspective)
        .add_bundle(antigen_wgpu::BufferDataBundle::new(
            PerspectiveMatrixComponent::construct(perspective_matrix(
                640.0 / 480.0,
                (0.0, 0.0),
                1.0,
                500.0,
            )),
            0,
            uniform_entity,
        ))
        .build();
    let _perspective_entity = world.spawn(bundle);

    // Orthographic matrix entity
    let mut builder = EntityBuilder::new();
    let bundle = builder
        .add(Orthographic)
        .add_bundle(antigen_wgpu::BufferDataBundle::new(
            OrthographicMatrixComponent::construct(orthographic_matrix(
                640.0 / 480.0,
                200.0,
                1.0,
                500.0,
            )),
            buffer_size_of::<[[f32; 4]; 4]>(),
            uniform_entity,
        ))
        .build();
    let _orthographic_entity = world.spawn(bundle);

    // Beam buffer texture
    let mut builder = EntityBuilder::new();
    let bundle = builder
        .add(BeamBuffer)
        .add_bundle(antigen_wgpu::TextureBundle::new(TextureDescriptor {
            label: Some("Beam Buffer"),
            size: Extent3d {
                width: 640,
                height: 480,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: TextureDimension::D2,
            format: HDR_TEXTURE_FORMAT,
            usage: TextureUsages::RENDER_ATTACHMENT | TextureUsages::TEXTURE_BINDING,
        }))
        .add_bundle(antigen_wgpu::TextureViewBundle::new(
            TextureViewDescriptor {
                label: Some("Beam Buffer View"),
                format: None,
                dimension: None,
                aspect: TextureAspect::All,
                base_mip_level: 0,
                mip_level_count: None,
                base_array_layer: 0,
                array_layer_count: None,
            },
        ))
        .build();
    let beam_buffer_entity = world.spawn(bundle);

    // Beam depth buffer
    let mut builder = EntityBuilder::new();
    let bundle = builder
        .add(BeamDepthBuffer)
        .add_bundle(antigen_wgpu::TextureBundle::new(TextureDescriptor {
            label: Some("Beam Depth Buffer"),
            size: Extent3d {
                width: 640,
                height: 480,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 4,
            dimension: TextureDimension::D2,
            format: TextureFormat::Depth32Float,
            usage: TextureUsages::RENDER_ATTACHMENT,
        }))
        .add_bundle(antigen_wgpu::TextureViewBundle::new(
            TextureViewDescriptor {
                label: Some("Beam Depth Buffer View"),
                format: None,
                dimension: None,
                aspect: TextureAspect::All,
                base_mip_level: 0,
                mip_level_count: None,
                base_array_layer: 0,
                array_layer_count: None,
            },
        ))
        .build();
    let beam_depth_buffer_entity = world.spawn(bundle);

    // Beam multisample resolve target
    let mut builder = EntityBuilder::new();
    let bundle = builder
        .add(BeamMultisample)
        .add_bundle(antigen_wgpu::TextureBundle::new(TextureDescriptor {
            label: Some("Beam Multisample"),
            size: Extent3d {
                width: 640,
                height: 480,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 4,
            dimension: TextureDimension::D2,
            format: HDR_TEXTURE_FORMAT,
            usage: TextureUsages::RENDER_ATTACHMENT,
        }))
        .add_bundle(antigen_wgpu::TextureViewBundle::new(
            TextureViewDescriptor {
                label: Some("Beam Multisample View"),
                format: None,
                dimension: None,
                aspect: TextureAspect::All,
                base_mip_level: 0,
                mip_level_count: None,
                base_array_layer: 0,
                array_layer_count: None,
            },
        ))
        .build();
    let beam_multisample_entity = world.spawn(bundle);

    // Phosphor buffers
    let phosphor_front_entity = world.reserve_entity();
    let phosphor_back_entity = world.reserve_entity();

    // Phosphor front buffer
    let mut builder = EntityBuilder::new();
    let bundle = builder
        .add(PhosphorFrontBuffer)
        .add(BindGroupComponent::default())
        .add_bundle(antigen_wgpu::TextureBundle::new(TextureDescriptor {
            label: Some("Phosphor Front Buffer"),
            size: Extent3d {
                width: 640,
                height: 480,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: TextureDimension::D2,
            format: HDR_TEXTURE_FORMAT,
            usage: TextureUsages::RENDER_ATTACHMENT | TextureUsages::TEXTURE_BINDING,
        }))
        .add_bundle(antigen_wgpu::TextureViewBundle::new(
            TextureViewDescriptor {
                label: Some("Phosphor Front Buffer View"),
                format: None,
                dimension: None,
                aspect: TextureAspect::All,
                base_mip_level: 0,
                mip_level_count: None,
                base_array_layer: 0,
                array_layer_count: None,
            },
        ))
        .add_bundle(
            antigen_core::swap_with_builder::<TextureViewComponent>(phosphor_back_entity).build(),
        )
        .add_bundle(
            antigen_core::swap_with_builder::<BindGroupComponent>(phosphor_back_entity).build(),
        )
        .build();
    world.insert(phosphor_front_entity, bundle).unwrap();

    // Phosphor back buffer
    let mut builder = EntityBuilder::new();
    let bundle = builder
        .add(PhosphorBackBuffer)
        .add(BindGroupComponent::default())
        .add_bundle(antigen_wgpu::TextureBundle::new(TextureDescriptor {
            label: Some("Phosphor Back Buffer"),
            size: Extent3d {
                width: 640,
                height: 480,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: TextureDimension::D2,
            format: HDR_TEXTURE_FORMAT,
            usage: TextureUsages::RENDER_ATTACHMENT | TextureUsages::TEXTURE_BINDING,
        }))
        .add_bundle(antigen_wgpu::TextureViewBundle::new(
            TextureViewDescriptor {
                label: Some("Phosphor Back Buffer View"),
                format: None,
                dimension: None,
                aspect: TextureAspect::All,
                base_mip_level: 0,
                mip_level_count: None,
                base_array_layer: 0,
                array_layer_count: None,
            },
        ))
        .build();
    world.insert(phosphor_back_entity, bundle).unwrap();

    // Assemble window
    let mut builder = EntityBuilder::new();
    let bundle = builder
        .add_bundle(antigen_winit::WindowBundle::default())
        .add_bundle(antigen_winit::WindowTitleBundle::new("Phosphor"))
        .add_bundle(antigen_wgpu::WindowSurfaceBundle::new())
        .add(RedrawUnconditionally)
        .build();
    world.insert(window_entity, bundle).unwrap();

    // Storage bind group
    let storage_bind_group_entity = world.spawn((
        StorageBuffers,
        BindGroupLayoutComponent::default(),
        BindGroupComponent::default(),
    ));

    // Beam mesh pass
    let beam_mesh_pass_entity = world.reserve_entity();
    let mut builder = EntityBuilder::new();
    builder.add(BeamMesh);
    builder.add(RenderPipelineComponent::default());
    world
        .insert(beam_mesh_pass_entity, builder.build())
        .unwrap();

    // Beam mesh draw indirect
    let triangle_indexed_indirect_builder = move |offset: u64| {
        let mut builder = EntityBuilder::new();

        builder.add(BeamMesh);

        builder.add_bundle(
            antigen_wgpu::RenderPassBundle::draw_indexed_indirect(
                0,
                Some("Beam Meshes".into()),
                vec![(
                    beam_multisample_entity,
                    Some(beam_buffer_entity),
                    Operations {
                        load: if offset == 0 {
                            LoadOp::Clear(CLEAR_COLOR)
                        } else {
                            LoadOp::Load
                        },
                        store: true,
                    },
                )],
                Some((
                    beam_depth_buffer_entity,
                    Some(Operations {
                        load: if offset == 0 {
                            LoadOp::Clear(1.0)
                        } else {
                            LoadOp::Load
                        },
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
    };

    // Beam line pass
    let beam_line_pass_entity = world.reserve_entity();
    let mut builder = EntityBuilder::new();
    builder.add(BeamLine);
    builder.add(RenderPipelineComponent::default());
    builder.add_bundle(
        antigen_wgpu::RenderPassBundle::draw(
            1,
            Some("Beam Lines".into()),
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
                    store: false,
                }),
                None,
            )),
            beam_line_pass_entity,
            vec![
                (line_vertex_entity, 0..224),
                (line_instance_entity, 0..960000),
            ],
            None,
            vec![
                (uniform_entity, vec![]),
                (storage_bind_group_entity, vec![0]),
            ],
            vec![],
            None,
            None,
            None,
            None,
            (0..14, 0..MAX_LINE_INSTANCES as u32),
            renderer_entity,
        )
        .build(),
    );
    world
        .insert(beam_line_pass_entity, builder.build())
        .unwrap();

    let beam_entity = world.spawn((Beam,));
    load_shader::<Filesystem, _>(
        channel,
        beam_entity,
        "crates/sandbox/src/demos/phosphor/shaders/beam.wgsl",
    );

    // Phosphor pass
    let phosphor_pass_entity = world.reserve_entity();
    let mut builder = EntityBuilder::new();
    builder.add(PhosphorDecay);
    builder.add(RenderPipelineComponent::default());
    builder.add(BindGroupLayoutComponent::default());
    builder.add_bundle(
        antigen_wgpu::RenderPassBundle::draw(
            2,
            Some("Phosphor Decay".into()),
            vec![(
                phosphor_front_entity,
                None,
                Operations {
                    load: LoadOp::Load,
                    store: true,
                },
            )],
            None,
            phosphor_pass_entity,
            vec![],
            None,
            vec![(uniform_entity, vec![]), (phosphor_front_entity, vec![])],
            vec![],
            None,
            None,
            None,
            None,
            (0..4, 0..1 as u32),
            renderer_entity,
        )
        .build(),
    );
    world.insert(phosphor_pass_entity, builder.build()).unwrap();

    load_shader::<Filesystem, _>(
        channel,
        phosphor_pass_entity,
        "crates/sandbox/src/demos/phosphor/shaders/phosphor_decay.wgsl",
    );

    // Tonemap pass
    let tonemap_pass_entity = world.reserve_entity();

    let mut builder = EntityBuilder::new();
    builder.add(Tonemap);
    builder.add(RenderPipelineComponent::default());
    builder.add_bundle(
        antigen_wgpu::RenderPassBundle::draw(
            3,
            Some("Tonemap".into()),
            vec![(
                window_entity,
                None,
                Operations {
                    load: LoadOp::Clear(Color::BLACK),
                    store: true,
                },
            )],
            None,
            tonemap_pass_entity,
            vec![],
            None,
            vec![(phosphor_back_entity, vec![])],
            vec![],
            None,
            None,
            None,
            None,
            (0..4, 0..1),
            renderer_entity,
        )
        .build(),
    );

    world.insert(tonemap_pass_entity, builder.build()).unwrap();

    load_shader::<Filesystem, _>(
        channel,
        tonemap_pass_entity,
        "crates/sandbox/src/demos/phosphor/shaders/tonemap.wgsl",
    );

    // Renderer
    let mut builder = EntityBuilder::new();

    builder.add(PhosphorRenderer);

    // Phosphor sampler
    builder.add_bundle(antigen_wgpu::SamplerBundle::new(SamplerDescriptor {
        label: Some("Linear Sampler"),
        address_mode_u: AddressMode::ClampToEdge,
        address_mode_v: AddressMode::ClampToEdge,
        address_mode_w: AddressMode::ClampToEdge,
        mag_filter: FilterMode::Linear,
        min_filter: FilterMode::Linear,
        mipmap_filter: FilterMode::Linear,
        ..Default::default()
    }));

    // Command encoder
    builder.add_bundle(antigen_wgpu::CommandEncoderBundle::new(
        CommandEncoderDescriptor {
            label: Some("Phosphor Encoder"),
        },
        renderer_entity,
    ));

    // Misc
    builder
        .add(antigen_wgpu::CommandBuffersComponent::default())
        // Indirect surface config and view for resize handling
        .add(Indirect::<&SurfaceConfigurationComponent>::construct(
            window_entity,
        ))
        .add(Indirect::<&TextureViewComponent>::construct(window_entity))
        // Indirect window for input handling
        .add(Indirect::<&WindowComponent>::construct(window_entity));

    // Done
    let bundle = builder.build();
    world.insert(renderer_entity, bundle).unwrap();

    // Load SVG meshes
    {
        let svg = SvgLayers::parse("crates/sandbox/src/demos/phosphor/fonts/basic.svg")
            .expect("Failed to parse SVG");
        let meshes = svg.meshes();
        for (_, graphemes) in meshes.iter() {
            for (grapheme, (vertices, indices)) in graphemes.iter() {
                let vertices = vertices
                    .into_iter()
                    .map(|(x, y)| VertexData {
                        position: [*x, -*y, 0.0],
                        surface_color: [0.0, 0.0, 0.0],
                        line_color: [1.0, 0.5, 0.0],
                        intensity: 0.5,
                        delta_intensity: -2.0,
                        ..Default::default()
                    })
                    .collect();

                let indices = indices
                    .into_iter()
                    .map(|index| *index as u32)
                    .collect::<Vec<_>>();

                let line_mesh = world
                    .query_one_mut::<&mut BufferLengthComponent>(line_mesh_entity)
                    .unwrap()
                    .load(Ordering::Relaxed) as u32;
                let line_count = indices.len() as u32 / 2;

                let key = format!("char_{}", grapheme);
                register_mesh_ids(world, &key, None, Some((line_mesh, line_count)));

                let mut builder = LineMeshBundle::builder(world, vertices, indices);
                let bundle = builder.build();

                world.spawn(bundle);
            }
        }
    }

    // Load box bot mesh
    let mut builders = BoxBotMeshBundle::builders(world, triangle_indexed_indirect_builder);
    let bundles = builders.iter_mut().map(EntityBuilder::build);
    world.extend(bundles);

    /*
    load_map::<MapFile, Filesystem, _>(
        channel,
        renderer_entity,
        "crates/sandbox/src/demos/phosphor/maps/index_align_test.map",
    );
    */

    assemble_test_geometry(world);

    channel
        .send_to::<Game>(insert_tagged_entity::<
            (&TriangleMeshInstances, &BufferComponent),
            TriangleMeshInstances,
        >())
        .unwrap();

    send_clone_query::<(&LineMeshInstances, &BufferComponent, &BufferLengthComponent), Game>(
        line_mesh_instance_entity,
    )((world, channel))
    .unwrap();

    channel
        .send_to::<Game>(insert_tagged_entity::<
            (&LineMeshInstances, &BufferComponent),
            LineMeshInstances,
        >())
        .unwrap();

    send_clone_query::<(&LineInstances, &BufferComponent, &BufferLengthComponent), Game>(
        line_instance_entity,
    )((world, channel))
    .unwrap();

    channel
        .send_to::<Game>(insert_tagged_entity::<
            (&LineInstances, &BufferComponent),
            LineInstances,
        >())
        .unwrap();

    // Load map file
    {
        let map_file = include_str!("maps/line_index_test.map");
        let map = map_file
            .parse::<antigen_shambler::shambler::shalrath::repr::Map>()
            .unwrap();
        let geo_map = GeoMap::from(map);
        let map_data = MapData::from(geo_map);

        let mut room_brushes = map_data.build_rooms(world, triangle_indexed_indirect_builder);
        let bundles = room_brushes.iter_mut().map(EntityBuilder::build);
        world.extend(bundles);

        let mut mesh_brushes = map_data.build_meshes(world, triangle_indexed_indirect_builder);
        let bundles = mesh_brushes.iter_mut().map(EntityBuilder::build);
        world.extend(bundles);

        // Clone mesh IDs to game thread
        send_clone_query::<(&MeshIds, &MeshIdsComponent), Game>(mesh_ids_entity)((world, channel))
            .unwrap();

        fn assemble_point_entities_message(
            map_data: MapData,
            triangle_indexed_indirect_builder: impl Fn(u64) -> EntityBuilder + Copy,
        ) -> impl for<'a, 'b> FnOnce(MessageContext<'a, 'b>) -> MessageResult<'a, 'b> {
            move |mut ctx| {
                let (world, _) = &mut ctx;
                println!("Assembling point entities on game thread");
                let mut point_entities =
                    map_data.build_point_entities(world, triangle_indexed_indirect_builder);
                let bundles = point_entities.iter_mut().map(EntityBuilder::build);
                world.extend(bundles);
                Ok(ctx)
            }
        }

        channel
            .send_to::<Game>(assemble_point_entities_message(
                map_data,
                triangle_indexed_indirect_builder,
            ))
            .unwrap();
    }
}

fn assemble_test_geometry(world: &mut World) {
    let (_, tagged_entities) = world
        .query_mut::<&TaggedEntitiesComponent>()
        .into_iter()
        .next()
        .unwrap();

    let line_mesh_id = std::any::TypeId::of::<LineMeshes>();
    let line_mesh_entity = tagged_entities[&line_mesh_id];

    // Oscilloscopes
    let mut builder = OscilloscopeMeshBundle::builder(
        world,
        "sin_cos_sin",
        RED,
        Oscilloscope::new(3.33, 30.0, |f| (f.sin(), f.cos(), f.sin())),
        2.0,
        -1.0,
    );
    let bundle = builder.build();
    world.spawn(bundle);

    let mut builder = OscilloscopeMeshBundle::builder(
        world,
        "sin_sin_cos",
        GREEN,
        Oscilloscope::new(2.22, 30.0, |f| (f.sin(), (f * 1.2).sin(), (f * 1.4).cos())),
        2.0,
        -2.0,
    );
    let bundle = builder.build();
    world.spawn(bundle);

    let mut builder = OscilloscopeMeshBundle::builder(
        world,
        "cos_cos_cos",
        BLUE,
        Oscilloscope::new(3.33, 30.0, |f| (f.cos(), (f * 1.2).cos(), (f * 1.4).cos())),
        2.0,
        -4.0,
    );
    world.spawn(builder.build());

    // Equilateral triangle
    let line_mesh = world
        .query_one_mut::<&mut BufferLengthComponent>(line_mesh_entity)
        .unwrap()
        .load(Ordering::Relaxed) as u32;
    let line_count = 4;

    let base_vert = nalgebra::vector![0.0, 45.0, 0.0];
    let vertices = (0..4).fold(vec![], |mut acc, next| {
        let vert = nalgebra::UnitQuaternion::new(
            nalgebra::vector![0.0, 0.0, 1.0] * (360.0f32 / 3.0).to_radians() * next as f32,
        ) * base_vert;

        let color = match next % 3 {
            0 => RED,
            1 => GREEN,
            2 => BLUE,
            _ => unreachable!(),
        };

        acc.push(VertexData::new(
            (vert.x, vert.y, vert.z),
            color,
            color,
            5.0,
            -20.0,
        ));
        acc
    });

    register_mesh_ids(
        world,
        "triangle_equilateral",
        None,
        Some((line_mesh, line_count)),
    );

    let mut builder = LineStripMeshBundle::builder(world, vertices);
    let bundle = builder.build();
    world.spawn(bundle);
}

struct MapData {
    geo_map: antigen_shambler::shambler::GeoMap,
    lines: antigen_shambler::shambler::line::Lines,
    brush_entities: antigen_shambler::shambler::brush::BrushEntities,
    face_brushes: antigen_shambler::shambler::face::FaceBrushes,
    entity_centers: antigen_shambler::shambler::entity::EntityCenters,
    face_planes: antigen_shambler::shambler::face::FacePlanes,
    brush_hulls: antigen_shambler::shambler::brush::BrushHulls,
    face_vertices: antigen_shambler::shambler::face::FaceVertices,
    face_duplicates: antigen_shambler::shambler::face::FaceDuplicates,
    face_centers: antigen_shambler::shambler::face::FaceCenters,
    face_indices: antigen_shambler::shambler::face::FaceIndices,
    face_triangle_indices: antigen_shambler::shambler::face::FaceTriangleIndices,
    face_lines: antigen_shambler::shambler::face::FaceLines,
    interior_faces: antigen_shambler::shambler::face::InteriorFaces,
    face_bases: antigen_shambler::shambler::face::FaceBases,
    face_face_containment: antigen_shambler::shambler::face::FaceFaceContainment,
    brush_face_containment: antigen_shambler::shambler::brush::BrushFaceContainment,
}

impl From<GeoMap> for MapData {
    fn from(geo_map: GeoMap) -> Self {
        // Reverse lookup tables for brush -> entity, face -> brush
        println!("Generating brush entities");
        let brush_entities =
            antigen_shambler::shambler::brush::brush_entities(&geo_map.entity_brushes);

        println!("Generating face brushes");
        let face_brushes = antigen_shambler::shambler::face::face_brushes(&geo_map.brush_faces);

        // Create geo planes from brush planes
        println!("Generating face planes");
        let face_planes = antigen_shambler::shambler::face::face_planes(&geo_map.face_planes);

        // Create per-brush hulls from brush planes
        println!("Generating brush hulls");
        let brush_hulls =
            antigen_shambler::shambler::brush::brush_hulls(&geo_map.brush_faces, &face_planes);

        // Generate face vertices
        println!("Generating face vertices");
        let (face_vertices, _) = antigen_shambler::shambler::face::face_vertices(
            &geo_map.brush_faces,
            &face_planes,
            &brush_hulls,
        );

        // Find duplicate faces
        println!("Generating face duplicates");
        let face_duplicates = antigen_shambler::shambler::face::face_duplicates(
            &geo_map.faces,
            &face_planes,
            &face_vertices,
        );

        // Generate centers
        println!("Generating face centers");
        let face_centers = antigen_shambler::shambler::face::face_centers(&face_vertices);

        println!("Generating brush centers");
        let brush_centers =
            antigen_shambler::shambler::brush::brush_centers(&geo_map.brush_faces, &face_centers);

        println!("Generating entity centers");
        let entity_centers = antigen_shambler::shambler::entity::entity_centers(
            &geo_map.entity_brushes,
            &brush_centers,
        );

        // Generate per-plane CCW face indices
        println!("Generating face indices");
        let face_indices = antigen_shambler::shambler::face::face_indices(
            &geo_map.face_planes,
            &face_planes,
            &face_vertices,
            &face_centers,
            antigen_shambler::shambler::face::FaceWinding::Clockwise,
        );

        println!("Generating face triangle indices");
        let face_triangle_indices =
            antigen_shambler::shambler::face::face_triangle_indices(&face_indices);

        println!("Generating lines");
        let (lines, face_lines) = antigen_shambler::shambler::line::lines(&face_indices);

        println!("Generating line duplicates");
        let line_duplicates = antigen_shambler::shambler::line::line_duplicates(
            &geo_map.brushes,
            &lines,
            &geo_map.brush_faces,
            &face_duplicates,
            &face_vertices,
            &face_lines,
        );

        println!("Generating interior faces");
        let interior_faces = antigen_shambler::shambler::face::interior_faces(
            &geo_map.brushes,
            &geo_map.brush_faces,
            &face_duplicates,
            &face_lines,
            &line_duplicates,
        );

        // Generate tangents
        println!("Generating face bases");
        let face_bases = antigen_shambler::shambler::face::face_bases(
            &geo_map.faces,
            &face_planes,
            &geo_map.face_offsets,
            &geo_map.face_angles,
            &geo_map.face_scales,
        );

        // Calculate face-face containment
        println!("Generating face-face containment");
        let face_face_containment = antigen_shambler::shambler::face::face_face_containment(
            &geo_map.faces,
            &lines,
            &face_planes,
            &face_bases,
            &face_vertices,
            &face_lines,
        );

        // Calculate brush-face containment
        println!("Generating brush-face containment");
        let brush_face_containment = antigen_shambler::shambler::brush::brush_face_containment(
            &geo_map.brushes,
            &geo_map.faces,
            &geo_map.brush_faces,
            &brush_hulls,
            &face_vertices,
        );

        MapData {
            brush_entities,
            face_brushes,
            geo_map,
            lines,
            entity_centers,
            face_planes,
            brush_hulls,
            face_vertices,
            face_duplicates,
            face_centers,
            face_indices,
            face_triangle_indices,
            face_lines,
            interior_faces,
            face_bases,
            face_face_containment,
            brush_face_containment,
        }
    }
}

impl MapData {
    fn classname_brushes<'a>(
        &'a self,
        classname: &'a str,
    ) -> impl Iterator<Item = (&'a EntityId, &'a Vec<BrushId>)> {
        self.geo_map
            .entity_brushes
            .iter()
            .filter(move |(entity, _)| {
                let properties = self.geo_map.entity_properties.get(entity).unwrap();
                properties
                    .iter()
                    .find(|p| p.key == "classname" && p.value == classname)
                    .is_some()
            })
    }

    fn entity_property<'a>(&'a self, entity: &EntityId, property: &str) -> Option<&Property> {
        let properties = self.geo_map.entity_properties.get(entity).unwrap();
        properties.iter().find(|p| p.key == property)
    }

    fn face_color(texture_name: &str) -> (f32, f32, f32) {
        if texture_name.contains("blood") {
            RED
        } else if texture_name.contains("green") {
            GREEN
        } else if texture_name.contains("blue") {
            BLUE
        } else {
            WHITE
        }
    }

    fn face_intensity(texture_name: &str) -> f32 {
        if texture_name.ends_with("3") {
            0.25
        } else if texture_name.ends_with("2") {
            0.375
        } else if texture_name.ends_with("1") {
            0.5
        } else {
            0.125
        }
    }

    fn entity_faces<'a>(&'a self, brushes: &'a [BrushId]) -> impl Iterator<Item = &'a FaceId> {
        self.geo_map
            .brush_faces
            .iter()
            .filter_map(|(brush_id, faces)| {
                if brushes.contains(brush_id) {
                    Some(faces)
                } else {
                    None
                }
            })
            .flatten()
    }

    fn face_texture(&self, face_id: &FaceId) -> &str {
        let texture_id = self.geo_map.face_textures[&face_id];
        &self.geo_map.textures[&texture_id]
    }

    fn face_vertices(
        &self,
        face_id: &FaceId,
        color: (f32, f32, f32),
        intensity: f32,
        scale_factor: f32,
    ) -> impl Iterator<Item = VertexData> + '_ {
        let face_vertices = &self.face_vertices[&face_id];
        face_vertices.iter().map(move |v| VertexData {
            position: [v.x * scale_factor, v.z * scale_factor, v.y * scale_factor],
            surface_color: [color.0 * 0.015, color.1 * 0.015, color.2 * 0.015],
            line_color: [color.0, color.1, color.2],
            intensity,
            delta_intensity: -30.0,
            ..Default::default()
        })
    }

    fn face_triangle_indices(
        &self,
        face_id: &FaceId,
        offset: u16,
    ) -> impl Iterator<Item = u16> + '_ {
        let face_triangle_indices = self.face_triangle_indices.get(&face_id).unwrap();
        face_triangle_indices
            .iter()
            .map(move |i| *i as u16 + offset)
    }

    fn face_line_indices(&self, face_id: &FaceId, offset: u32) -> impl Iterator<Item = u32> + '_ {
        let face_lines = &self.face_lines[&face_id];
        face_lines.iter().flat_map(move |line_id| {
            let antigen_shambler::shambler::line::Line { i0, i1 } = self.lines[line_id];
            [(i0 + offset as usize) as u32, (i1 + offset as usize) as u32]
        })
    }

    pub fn build_rooms(
        &self,
        world: &mut World,
        triangle_indexed_indirect_builder: impl Fn(u64) -> EntityBuilder + Copy,
    ) -> Vec<EntityBuilder> {
        let mut builders = vec![];

        let entity_brushes = self
            .classname_brushes("room")
            .chain(self.classname_brushes("portal"));

        let (_, tagged_entities) = world
            .query_mut::<&TaggedEntitiesComponent>()
            .into_iter()
            .next()
            .unwrap();

        let vertex_id = std::any::TypeId::of::<Vertices>();
        let vertex_entity = tagged_entities[&vertex_id];

        let triangle_index_id = std::any::TypeId::of::<TriangleIndices>();
        let triangle_index_entity = tagged_entities[&triangle_index_id];

        let triangle_mesh_id = std::any::TypeId::of::<TriangleMeshes>();
        let triangle_mesh_entity = tagged_entities[&triangle_mesh_id];

        let line_index_id = std::any::TypeId::of::<LineIndices>();
        let line_index_entity = tagged_entities[&line_index_id];

        let line_mesh_id = std::any::TypeId::of::<LineMeshes>();
        let line_mesh_entity = tagged_entities[&line_mesh_id];

        for (entity, brushes) in entity_brushes {
            let entity_faces = self.entity_faces(brushes);
            let entity_center = self.entity_centers[entity];
            let entity_center = entity_center.xzy();

            // Generate mesh
            let mut mesh_vertices: Vec<VertexData> = Default::default();
            let mut triangle_indices: Vec<TriangleIndexData> = Default::default();
            let mut line_indices: Vec<LineIndexData> = Default::default();

            let scale_factor = 1.0;

            // Gather mesh and line geometry
            let base_vertex = world
                .query_one_mut::<&mut BufferLengthComponent>(vertex_entity)
                .unwrap()
                .load(Ordering::Relaxed) as u32;

            let base_triangle_index = world
                .query_one_mut::<&mut BufferLengthComponent>(triangle_index_entity)
                .unwrap()
                .load(Ordering::Relaxed) as u32;

            let triangle_mesh = world
                .query_one_mut::<&mut BufferLengthComponent>(triangle_mesh_entity)
                .unwrap()
                .load(Ordering::Relaxed) as u32;

            let base_line_index = world
                .query_one_mut::<&mut BufferLengthComponent>(line_index_entity)
                .unwrap()
                .load(Ordering::Relaxed) as u32;

            let line_mesh = world
                .query_one_mut::<&mut BufferLengthComponent>(line_mesh_entity)
                .unwrap()
                .load(Ordering::Relaxed) as u32;

            let mut local_vertex_head = 0u16;
            let mut local_index_head = 0u32;

            for face_id in entity_faces {
                if self.face_duplicates.iter().any(|(a, _)| a == face_id) {
                    continue;
                }

                if self
                    .face_face_containment
                    .iter()
                    .any(|(_, b)| b.contains(face_id))
                {
                    continue;
                }

                if self
                    .brush_face_containment
                    .iter()
                    .any(|(_, b)| b.contains(face_id))
                {
                    continue;
                }

                if !self.interior_faces.contains(&face_id) {
                    continue;
                }

                // Fetch and interpret texture data
                let texture_name = self.face_texture(&face_id);
                let color = Self::face_color(texture_name);
                let intensity = Self::face_intensity(texture_name);

                let verts = self
                    .face_vertices(face_id, color, intensity, scale_factor)
                    .map(|vertex| VertexData {
                        position: [
                            vertex.position[0] - entity_center[0],
                            vertex.position[1] - entity_center[1],
                            vertex.position[2] - entity_center[2],
                        ],
                        ..vertex
                    })
                    .collect::<Vec<_>>();
                let vertex_count = verts.len();
                mesh_vertices.extend(verts);

                triangle_indices.extend(self.face_triangle_indices(face_id, local_vertex_head));
                line_indices.extend(self.face_line_indices(face_id, local_index_head));

                local_vertex_head += vertex_count as u16;
                local_index_head += vertex_count as u32;
            }

            let vertex_count = mesh_vertices.len() as u32;
            let triangle_index_count = triangle_indices.len() as u32;
            let line_index_count = line_indices.len() as u32;

            let line_count = line_index_count / 2;

            // Mesh entity
            let mut builder = EntityBuilder::new();

            builder.add_bundle(
                TriangleMeshBundle::builder(world, mesh_vertices, triangle_indices).build(),
            );

            builder.add_bundle(
                TriangleMeshDataBundle::builder(
                    world,
                    triangle_index_count,
                    0,
                    base_triangle_index,
                    base_vertex,
                    triangle_indexed_indirect_builder,
                )
                .build(),
            );

            builder.add_bundle(LineIndicesBundle::builder(world, line_indices).build());

            builder.add_bundle(
                LineMeshDataBundle::builder(
                    world,
                    base_vertex,
                    vertex_count,
                    base_line_index,
                    line_index_count,
                )
                .build(),
            );

            builders.push(builder);

            let key = format!("entity_{}", entity);
            register_mesh_ids(
                world,
                &key,
                Some(triangle_mesh),
                Some((line_mesh, line_count)),
            );

            builders.extend(mesh_instance_builders(
                world,
                &key,
                entity_center.into(),
                nalgebra::Quaternion::identity().into(),
                nalgebra::vector![1.0, 1.0, 1.0].into(),
            ));
        }

        builders
    }

    pub fn build_meshes(
        &self,
        world: &mut World,
        triangle_indexed_indirect_builder: impl Fn(u64) -> EntityBuilder + Copy,
    ) -> Vec<EntityBuilder> {
        let mut builders = vec![];

        let entity_brushes = self.classname_brushes("mesh");

        let (_, tagged_entities) = world
            .query_mut::<&TaggedEntitiesComponent>()
            .into_iter()
            .next()
            .unwrap();

        let vertex_id = std::any::TypeId::of::<Vertices>();
        let vertex_entity = tagged_entities[&vertex_id];

        let triangle_index_id = std::any::TypeId::of::<TriangleIndices>();
        let triangle_index_entity = tagged_entities[&triangle_index_id];

        let triangle_mesh_id = std::any::TypeId::of::<TriangleMeshes>();
        let triangle_mesh_entity = tagged_entities[&triangle_mesh_id];

        let line_index_id = std::any::TypeId::of::<LineIndices>();
        let line_index_entity = tagged_entities[&line_index_id];

        let line_mesh_id = std::any::TypeId::of::<LineMeshes>();
        let line_mesh_entity = tagged_entities[&line_mesh_id];

        for (entity, brushes) in entity_brushes {
            let properties = self.geo_map.entity_properties.get(entity).unwrap();
            let entity_mesh_name = Self::property_targetname(properties);

            let entity_faces = self.entity_faces(brushes);
            let entity_center = self.entity_centers[entity];

            // Generate mesh
            let mut mesh_vertices: Vec<VertexData> = Default::default();
            let mut triangle_indices: Vec<TriangleIndexData> = Default::default();
            let mut line_indices: Vec<LineIndexData> = Default::default();

            let scale_factor = 1.0;

            // Gather mesh and line geometry
            let base_vertex = world
                .query_one_mut::<&BufferLengthComponent>(vertex_entity)
                .unwrap()
                .load(Ordering::Relaxed) as u32;

            let base_triangle_index = world
                .query_one_mut::<&mut BufferLengthComponent>(triangle_index_entity)
                .unwrap()
                .load(Ordering::Relaxed) as u32;

            let triangle_mesh = world
                .query_one_mut::<&mut BufferLengthComponent>(triangle_mesh_entity)
                .unwrap()
                .load(Ordering::Relaxed) as u32;

            let base_line_index = world
                .query_one_mut::<&mut BufferLengthComponent>(line_index_entity)
                .unwrap()
                .load(Ordering::Relaxed) as u32;

            let line_mesh = world
                .query_one_mut::<&mut BufferLengthComponent>(line_mesh_entity)
                .unwrap()
                .load(Ordering::Relaxed) as u32;

            let mut local_vertex_head = 0u16;
            let mut local_index_head = 0u32;

            for face_id in entity_faces {
                if self.face_duplicates.iter().any(|(_, b)| b == face_id) {
                    continue;
                }

                if self
                    .face_face_containment
                    .iter()
                    .any(|(_, b)| b.contains(face_id))
                {
                    continue;
                }

                if self
                    .brush_face_containment
                    .iter()
                    .any(|(_, b)| b.contains(face_id))
                {
                    continue;
                }

                // Fetch and interpret texture data
                let texture_name = self.face_texture(&face_id);
                let color = Self::face_color(texture_name);
                let intensity = Self::face_intensity(texture_name);

                let verts = self
                    .face_vertices(face_id, color, intensity, scale_factor)
                    .map(|vertex| VertexData {
                        position: [
                            vertex.position[0] - entity_center[0],
                            vertex.position[1] - entity_center[2],
                            vertex.position[2] - entity_center[1],
                        ],
                        ..vertex
                    })
                    .collect::<Vec<_>>();
                let vertex_count = verts.len();
                mesh_vertices.extend(verts);

                triangle_indices.extend(self.face_triangle_indices(face_id, local_vertex_head));
                line_indices.extend(self.face_line_indices(face_id, local_index_head));

                local_vertex_head += vertex_count as u16;
                local_index_head += vertex_count as u32;
            }

            let vertex_count = mesh_vertices.len() as u32;
            let triangle_index_count = triangle_indices.len() as u32;
            let line_index_count = line_indices.len() as u32;

            // Singleton mesh instance
            builders.extend([
                TriangleMeshBundle::builder(world, mesh_vertices, triangle_indices),
                TriangleMeshDataBundle::builder(
                    world,
                    triangle_index_count,
                    0,
                    base_triangle_index,
                    base_vertex,
                    triangle_indexed_indirect_builder,
                ),
                LineIndicesBundle::builder(world, line_indices),
                LineMeshDataBundle::builder(
                    world,
                    base_vertex,
                    vertex_count,
                    base_line_index,
                    line_index_count,
                ),
            ]);

            register_mesh_ids(
                world,
                &entity_mesh_name,
                Some(triangle_mesh),
                Some((line_mesh as u32, line_index_count / 2)),
            );
        }

        // Load oscilloscope meshes
        let oscilloscope_entities = self.geo_map.point_entities.iter().flat_map(|point_entity| {
            let properties = self.geo_map.entity_properties.get(point_entity)?;
            if let Some(classname) = properties.0.iter().find(|p| p.key == "classname") {
                if classname.value == "oscilloscope" {
                    Some(properties)
                } else {
                    None
                }
            } else {
                None
            }
        });

        for oscilloscope in oscilloscope_entities.into_iter() {
            let color = Self::property_f32_3("color", oscilloscope).unwrap();
            let intensity = Self::property_f32("intensity", oscilloscope).unwrap();
            let delta_intensity = Self::property_f32("delta_intensity", oscilloscope).unwrap();
            let speed = Self::property_f32("speed", oscilloscope).unwrap();
            let magnitude = Self::property_f32("magnitude", oscilloscope).unwrap();
            let targetname = Self::property_targetname(oscilloscope);

            let x = Self::property_expression_f32("x", oscilloscope).unwrap();
            let y = Self::property_expression_f32("y", oscilloscope).unwrap();
            let z = Self::property_expression_f32("z", oscilloscope).unwrap();

            builders.push(OscilloscopeMeshBundle::builder(
                world,
                targetname,
                color,
                Oscilloscope::new(speed, magnitude, move |f| {
                    let vars = [("f", f)].into_iter().collect::<BTreeMap<_, _>>();
                    (x.eval(&vars), y.eval(&vars), z.eval(&vars))
                }),
                intensity,
                delta_intensity,
            ));
        }

        builders
    }

    fn property_origin(properties: &Properties) -> nalgebra::Vector3<f32> {
        let (x, z, y) = Self::property_f32_3("origin", properties).unwrap();
        nalgebra::vector![x, y, z]
    }

    fn property_rotation(properties: &Properties, convert: bool) -> nalgebra::UnitQuaternion<f32> {
        let y_ofs = if convert { -90.0f32.to_radians() } else { 0.0 };
        if let Ok((x, y, z)) = Self::property_f32_3("mangle", properties) {
            nalgebra::UnitQuaternion::from_euler_angles(
                -z.to_radians(),
                -y.to_radians() + y_ofs,
                -x.to_radians(),
            )
        } else if let Ok(y) = Self::property_f32("angle", properties) {
            nalgebra::UnitQuaternion::from_euler_angles(0.0, -y.to_radians() + y_ofs, 0.0)
        } else {
            nalgebra::UnitQuaternion::default()
        }
    }

    fn property_scale(properties: &Properties) -> nalgebra::Vector3<f32> {
        if let Ok((x, z, y)) = Self::property_f32_3("scale", properties) {
            nalgebra::vector![x, z, y]
        } else {
            nalgebra::vector![1.0, 1.0, 1.0]
        }
    }

    fn property_targetname(properties: &Properties) -> &str {
        Self::property_string("targetname", properties).unwrap()
    }

    fn property_f32_3(
        key: &str,
        properties: &Properties,
    ) -> Result<(f32, f32, f32), Box<dyn Error>> {
        let property = properties
            .0
            .iter()
            .find(|p| p.key == key)
            .ok_or("Key not found")?;

        let mut value = property.value.split_whitespace();
        let x = value.next().unwrap().parse::<f32>()?;
        let y = value.next().unwrap().parse::<f32>()?;
        let z = value.next().unwrap().parse::<f32>()?;
        Ok((x, y, z))
    }

    fn property_f32(key: &str, properties: &Properties) -> Result<f32, Box<dyn Error>> {
        Ok(properties
            .0
            .iter()
            .find(|p| p.key == key)
            .ok_or("Key not found")?
            .value
            .parse::<f32>()?)
    }

    fn property_expression_f32(
        key: &str,
        properties: &Properties,
    ) -> Result<Expression<f32>, Box<dyn Error>> {
        let value = properties
            .0
            .iter()
            .find(|p| p.key == key)
            .ok_or("Key not found")?
            .value
            .as_str();
        Ok(expression::parse_expression(value))
    }

    fn property_string<'a>(
        key: &str,
        properties: &'a Properties,
    ) -> Result<&'a str, Box<dyn Error>> {
        Ok(properties
            .0
            .iter()
            .find(|p| p.key == key)
            .ok_or("Key not found")?
            .value
            .as_str())
    }

    pub fn build_point_entities(
        &self,
        world: &mut World,
        triangle_indexed_indirect_builder: impl Fn(u64) -> EntityBuilder + Copy,
    ) -> Vec<EntityBuilder> {
        let mut builders = vec![];

        // Spawn player start entities
        let player_start_entities = self.geo_map.point_entities.iter().flat_map(|point_entity| {
            let properties = self.geo_map.entity_properties.get(point_entity)?;
            if let Some(classname) = properties.0.iter().find(|p| p.key == "classname") {
                if classname.value == "info_player_start" {
                    Some(properties)
                } else {
                    None
                }
            } else {
                None
            }
        });

        for player_start in player_start_entities.into_iter() {
            let origin = Self::property_origin(player_start);
            let rotation = Self::property_rotation(player_start, true);
            let scale = Self::property_scale(player_start);

            builders.extend(mesh_instance_builders(
                world,
                "box_bot",
                origin.into(),
                rotation.into_inner().into(),
                scale.into(),
            ));
        }

        // Spawn oscilloscope entities
        let oscilloscope_entities = self.geo_map.point_entities.iter().flat_map(|point_entity| {
            let properties = self.geo_map.entity_properties.get(point_entity)?;
            if let Some(classname) = properties.0.iter().find(|p| p.key == "classname") {
                if classname.value == "oscilloscope" {
                    Some(properties)
                } else {
                    None
                }
            } else {
                None
            }
        });

        for oscilloscope in oscilloscope_entities.into_iter() {
            let origin = Self::property_origin(oscilloscope);
            let targetname = Self::property_targetname(oscilloscope);

            builders.extend(mesh_instance_builders(
                world,
                &format!("oscilloscope_{}", targetname),
                origin.into(),
                nalgebra::Quaternion::identity().into(),
                nalgebra::vector![1.0, 1.0, 1.0].into(),
            ));
        }

        // Spawn mesh instance entities
        let mesh_instance_entities = self.geo_map.point_entities.iter().flat_map(|point_entity| {
            let properties = self.geo_map.entity_properties.get(point_entity)?;
            if let Some(classname) = properties.0.iter().find(|p| p.key == "classname") {
                if classname.value == "mesh_instance" {
                    Some(properties)
                } else {
                    None
                }
            } else {
                None
            }
        });

        for properties in mesh_instance_entities.into_iter() {
            let origin = Self::property_origin(properties);
            let rotation = Self::property_rotation(properties, false);
            let scale = Self::property_scale(properties);

            let target = Self::property_string("target", properties).unwrap();

            builders.extend(mesh_instance_builders(
                world,
                target,
                origin.into(),
                rotation.into_inner().into(),
                scale.into(),
            ));
        }

        // Spawn text entities
        let text_entities = self.geo_map.point_entities.iter().flat_map(|point_entity| {
            let properties = self.geo_map.entity_properties.get(point_entity)?;
            if let Some(classname) = properties.0.iter().find(|p| p.key == "classname") {
                if classname.value == "text" {
                    Some(properties)
                } else {
                    None
                }
            } else {
                None
            }
        });

        for properties in text_entities.into_iter() {
            let origin = Self::property_origin(properties);
            let rotation = Self::property_rotation(properties, true);
            let scale = Self::property_scale(properties);

            let text = Self::property_string("text", properties).unwrap();

            let lines = text
                .split("\\n")
                .map(|line| line.chars().collect::<Vec<_>>())
                .collect::<Vec<_>>();

            let step = 20.0;
            for (iy, chars) in lines.iter().enumerate() {
                for (ix, c) in chars.iter().enumerate() {
                    if *c == ' ' {
                        continue;
                    }

                    let ofs = nalgebra::vector![
                        (-step * 13.0) + ix as f32 * 20.0,
                        (iy as f32 * -30.0),
                        0.0
                    ];
                    let ofs = ofs.component_mul(&scale);
                    let ofs = rotation * ofs;

                    let key = format!("char_{}", c.to_string().as_str());
                    builders.extend(mesh_instance_builders(
                        world,
                        &key,
                        (origin + ofs).into(),
                        rotation.into_inner().into(),
                        scale.into(),
                    ));
                }
            }
        }

        builders
    }
}

pub fn winit_event_handler<T>(mut f: impl EventLoopHandler<T>) -> impl EventLoopHandler<T> {
    fn prepare_schedule(world: &mut World) {
        // parallel
        {
            antigen_wgpu::create_shader_modules_system(world);
            antigen_wgpu::create_buffers_system(world);
            antigen_wgpu::create_textures_system(world);
            antigen_wgpu::create_texture_views_system(world);
            antigen_wgpu::create_samplers_system(world);
        }
        //parallel
        {
            antigen_wgpu::buffer_write_system::<TotalTimeComponent>(world);
            antigen_wgpu::buffer_write_system::<DeltaTimeComponent>(world);
            antigen_wgpu::buffer_write_system::<PerspectiveMatrixComponent>(world);
            antigen_wgpu::buffer_write_system::<OrthographicMatrixComponent>(world);
            antigen_wgpu::buffer_write_slice_system::<VertexDataComponent, _>(world);
            antigen_wgpu::buffer_write_slice_system::<TriangleIndexDataComponent, _>(world);
            antigen_wgpu::buffer_write_slice_system::<TriangleMeshDataComponent, _>(world);
            antigen_wgpu::buffer_write_system::<PositionComponent>(world);
            antigen_wgpu::buffer_write_system::<RotationComponent>(world);
            antigen_wgpu::buffer_write_system::<ScaleComponent>(world);
            antigen_wgpu::buffer_write_system::<LineMeshIdComponent>(world);
            antigen_wgpu::buffer_write_slice_system::<TriangleMeshInstanceDataComponent, _>(world);
            antigen_wgpu::buffer_write_slice_system::<LineVertexDataComponent, _>(world);
            antigen_wgpu::buffer_write_slice_system::<LineIndexDataComponent, _>(world);
            antigen_wgpu::buffer_write_slice_system::<LineMeshDataComponent, _>(world);
            antigen_wgpu::buffer_write_slice_system::<LineMeshInstanceDataComponent, _>(world);
            antigen_wgpu::buffer_write_slice_system::<LineInstanceDataComponent, _>(world);
        }
        phosphor_update_beam_mesh_draw_count_system(world);
        phosphor_update_beam_line_draw_count_system(world);
        phosphor_prepare_system(world);
    }

    fn render_schedule(world: &mut World) {
        //parallel
        {
            phosphor_update_total_time_system(world);
            phosphor_update_delta_time_system(world);
        }
        phosphor_update_oscilloscopes_system(world);
        antigen_wgpu::create_command_encoders_system(world);
        antigen_wgpu::draw_render_passes_system(world);
        antigen_core::swap_with_system::<TextureViewComponent>(world);
        antigen_core::swap_with_system::<BindGroupComponent>(world);
        antigen_wgpu::flush_command_encoders_system(world);
        phosphor_update_timestamp_system(world);
        antigen_wgpu::device_poll_system(&Maintain::Wait)(world);
    }

    move |world: &mut World,
          channel: &WorldChannel,
          event: Event<'static, T>,
          event_loop_window_target: &EventLoopWindowTarget<T>,
          control_flow: &mut ControlFlow| {
        match &event {
            Event::MainEventsCleared => {
                phosphor_resize_system(world);
                prepare_schedule(world);
            }
            Event::WindowEvent { event, .. } => match event {
                WindowEvent::Resized(_) => {
                    phosphor_resize_system(world);
                }
                WindowEvent::CursorMoved { .. } => phosphor_cursor_moved_system(world),
                _ => (),
            },
            Event::RedrawEventsCleared => {
                render_schedule(world);
            }
            _ => (),
        }

        f(
            world,
            channel,
            event,
            event_loop_window_target,
            control_flow,
        );
    }
}
