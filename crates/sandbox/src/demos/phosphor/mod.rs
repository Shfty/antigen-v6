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
//       [>] Investigate infinite perspective projection + reversed Z
//           [✓] Implement new matrix
//           [ ] Fix triangle-line Z-fighting
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

use antigen_fs::{load_file_string, FilePathComponent, FileStringQuery};
use antigen_rapier3d::{
    AngularVelocityComponent, ColliderComponent, LinearVelocityComponent, RigidBodyComponent,
};
pub use assemblage::*;
pub use components::*;
use rapier3d::prelude::{
    ActiveEvents, ColliderBuilder, IntersectionEvent, RigidBodyBuilder, SharedShape,
};
pub use render_passes::*;
pub use svg_lines::*;
pub use systems::*;

use expression::{EvalTrait, Expression};
use std::{
    borrow::Cow, collections::BTreeMap, error::Error, path::PathBuf, sync::atomic::Ordering,
    time::Instant,
};
use winit::event::DeviceEvent;

use antigen_winit::{
    winit::{
        event::{Event, WindowEvent},
        event_loop::{ControlFlow, EventLoopWindowTarget},
    },
    EventLoopHandler, RedrawUnconditionally, WindowComponent,
};

use antigen_core::{
    get_tagged_entity, insert_tagged_entity, insert_tagged_entity_by_query, send_clone_query,
    send_component, Construct, Indirect, Lift, MessageContext, MessageResult, NamedEntityComponent,
    PositionComponent, RotationComponent, ScaleComponent, SendTo, WorldChannel,
};

use antigen_wgpu::{
    buffer_size_of, spawn_shader_from_file_string,
    wgpu::{
        AddressMode, BufferAddress, BufferDescriptor, BufferUsages, Color,
        CommandEncoderDescriptor, Extent3d, FilterMode, LoadOp, Maintain, Operations,
        SamplerDescriptor, TextureAspect, TextureDescriptor, TextureDimension, TextureFormat,
        TextureUsages, TextureViewDescriptor,
    },
    BindGroupComponent, BindGroupLayoutComponent, BufferComponent, BufferLengthComponent,
    BufferLengthsComponent, RenderPipelineComponent, ShaderModuleComponent,
    ShaderModuleDescriptorComponent, SurfaceConfigurationComponent, TextureViewComponent,
};

use antigen_shambler::shambler::{
    brush::BrushId,
    entity::EntityId,
    face::FaceId,
    line::LineId,
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
const NEAR_PLANE: f32 = 5.0;

pub const BLACK: (f32, f32, f32) = (0.0, 0.0, 0.0);
pub const RED: (f32, f32, f32) = (1.0, 0.0, 0.0);
pub const GREEN: (f32, f32, f32) = (0.0, 1.0, 0.0);
pub const BLUE: (f32, f32, f32) = (0.0, 0.0, 1.0);
pub const WHITE: (f32, f32, f32) = (1.0, 1.0, 1.0);

pub fn orthographic_matrix(aspect: f32, zoom: f32) -> nalgebra::Matrix4<f32> {
    let mut ortho =
        nalgebra_glm::ortho_rh_zo(-zoom * aspect, zoom * aspect, -zoom, zoom, 0.0, zoom * 50.0);
    ortho.append_nonuniform_scaling_mut(&nalgebra::vector![1.0, 1.0, -1.0]);
    ortho
}

pub fn perspective_matrix(aspect: f32, near: f32) -> nalgebra::Matrix4<f32> {
    nalgebra_glm::reversed_infinite_perspective_rh_zo(aspect, (70.0f32).to_radians(), near)
}

fn circle_strip(subdiv: usize, z_ofs: f32) -> Vec<LineVertexData> {
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
            position: [x, y, z_ofs],
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

fn load_map<
    U: Send + Sync + 'static,
    T: Send + Sync + 'static,
    P: Copy + Into<PathBuf> + Send + Sync + 'static,
>(
    channel: &WorldChannel,
    map_path: P,
) {
    channel
        .send_to::<T>(load_map_message::<U, _>(map_path))
        .unwrap();
}

fn load_map_message<U: Send + Sync + 'static, P: Copy + Into<PathBuf>>(
    map_path: P,
) -> impl for<'a, 'b> FnOnce(MessageContext<'a, 'b>) -> MessageResult<'a, 'b> {
    move |ctx| {
        ctx.lift()
            .and_then(load_file_string(map_path))
            .and_then(parse_map_file_string(map_path))
    }
}

pub fn parse_map_file_string<'a, 'b, P: Into<PathBuf>>(
    path: P,
) -> impl FnOnce(MessageContext<'a, 'b>) -> MessageResult<'a, 'b> {
    move |mut ctx| {
        let (world, channel) = &mut ctx;

        let map_path = path.into();
        println!(
            "Thread {} Looking for file string entities with path {:?}..",
            std::thread::current().name().unwrap(),
            map_path
        );

        let (entity, FileStringQuery { string, .. }) = world
            .query_mut::<FileStringQuery>()
            .into_iter()
            .filter(|(_, FileStringQuery { path, .. })| ***path == *map_path)
            .next()
            .unwrap();

        println!("Parsing map file for entity {:?}", entity);
        let map = string
            .parse::<antigen_shambler::shambler::shalrath::repr::Map>()
            .unwrap();
        let geo_map = GeoMap::from(map);
        let map_data = MapData::from(geo_map);

        channel
            .send_to::<Render>(assemble_map_render_thread(map_data.clone()))
            .unwrap();

        channel
            .send_to::<Game>(assemble_map_game_thread(map_data))
            .unwrap();

        Ok(ctx)
    }
}

fn assemble_map_render_thread(
    map_data: MapData,
) -> impl for<'a, 'b> FnOnce(MessageContext<'a, 'b>) -> MessageResult<'a, 'b> {
    move |mut ctx| {
        let (world, _) = &mut ctx;

        let mut map_meshes = map_data.assemble_brush_entities_render_thread(world);
        let bundles = map_meshes.iter_mut().map(EntityBuilder::build);
        world.extend(bundles);

        let mut map_meshes = map_data.assemble_point_entities_render_thread(world);
        let bundles = map_meshes.iter_mut().map(EntityBuilder::build);
        world.extend(bundles);

        Ok(ctx)
    }
}

fn assemble_map_game_thread(
    map_data: MapData,
) -> impl for<'a, 'b> FnOnce(MessageContext<'a, 'b>) -> MessageResult<'a, 'b> {
    move |mut ctx| {
        let (world, _) = &mut ctx;

        map_data.assemble_brush_entities_game_thread(world);

        let mut point_entities = map_data.assemble_entities_game_thread(world);
        let bundles = point_entities.iter_mut().map(EntityBuilder::build);
        world.extend(bundles);

        Ok(ctx)
    }
}

fn insert_tagged_entity_by_query_message<Q: hecs::Query + Send + Sync + 'static, T: 'static>(
) -> impl for<'a, 'b> Fn(MessageContext<'a, 'b>) -> Result<MessageContext<'a, 'b>, Box<dyn Error>> {
    move |mut ctx: MessageContext| {
        let (world, _) = &mut ctx;
        insert_tagged_entity_by_query::<Q, T>(world);
        Ok(ctx)
    }
}

// Bundles
fn uniform_buffer_bundle() -> EntityBuilder {
    let mut builder = EntityBuilder::new();
    builder
        .add(Uniform)
        .add(BindGroupLayoutComponent::default())
        .add(BindGroupComponent::default())
        .add_bundle(antigen_wgpu::BufferBundle::new(BufferDescriptor {
            label: Some("Uniform Buffer"),
            size: buffer_size_of::<UniformData>(),
            usage: BufferUsages::UNIFORM | BufferUsages::INDIRECT | BufferUsages::COPY_DST,
            mapped_at_creation: false,
        }));
    builder
}

fn vertex_buffer_bundle() -> EntityBuilder {
    let mut builder = EntityBuilder::new();
    builder
        .add(Vertices)
        .add_bundle(antigen_wgpu::BufferBundle::new(BufferDescriptor {
            label: Some("Vertex Buffer"),
            size: buffer_size_of::<VertexData>() * MAX_MESH_VERTICES as BufferAddress,
            usage: BufferUsages::VERTEX | BufferUsages::STORAGE | BufferUsages::COPY_DST,
            mapped_at_creation: false,
        }))
        .add(BufferLengthComponent::default());
    builder
}

fn triangle_index_buffer_bundle() -> EntityBuilder {
    let mut builder = EntityBuilder::new();
    builder
        .add(TriangleIndices)
        .add_bundle(antigen_wgpu::BufferBundle::new(BufferDescriptor {
            label: Some("Triangle Index Buffer"),
            size: buffer_size_of::<TriangleIndexData>() * MAX_TRIANGLE_INDICES as BufferAddress,
            usage: BufferUsages::INDEX | BufferUsages::COPY_DST,
            mapped_at_creation: false,
        }))
        .add(BufferLengthComponent::default());
    builder
}

fn triangle_mesh_buffer_bundle() -> EntityBuilder {
    let mut builder = EntityBuilder::new();
    builder
        .add(TriangleMeshes)
        .add_bundle(antigen_wgpu::BufferBundle::new(BufferDescriptor {
            label: Some("Triangle Mesh Buffer"),
            size: buffer_size_of::<TriangleMeshData>() * MAX_TRIANGLE_MESHES as BufferAddress,
            usage: BufferUsages::INDIRECT | BufferUsages::COPY_DST,
            mapped_at_creation: false,
        }))
        .add(BufferLengthComponent::default());
    builder
}

fn triangle_mesh_instances_buffer_bundle() -> EntityBuilder {
    let mut builder = EntityBuilder::new();
    builder
        .add(TriangleMeshInstances)
        .add_bundle(antigen_wgpu::BufferBundle::new(BufferDescriptor {
            label: Some("Triangle Mesh Instance Buffer"),
            size: buffer_size_of::<TriangleMeshInstanceData>()
                * (MAX_TRIANGLE_MESHES * MAX_TRIANGLE_MESH_INSTANCES) as BufferAddress,
            usage: BufferUsages::STORAGE | BufferUsages::COPY_DST,
            mapped_at_creation: false,
        }))
        .add(BufferLengthsComponent::default());
    builder
}

fn line_index_buffer_bundle() -> EntityBuilder {
    let mut builder = EntityBuilder::new();
    builder
        .add(LineIndices)
        .add_bundle(antigen_wgpu::BufferBundle::new(BufferDescriptor {
            label: Some("Line Index Buffer"),
            size: buffer_size_of::<LineIndexData>() * MAX_LINE_INDICES as BufferAddress,
            usage: BufferUsages::STORAGE | BufferUsages::COPY_DST,
            mapped_at_creation: false,
        }))
        .add(BufferLengthComponent::default());
    builder
}

fn mesh_buffer_bundle() -> EntityBuilder {
    let mut builder = EntityBuilder::new();
    builder
        .add(LineMeshes)
        .add_bundle(antigen_wgpu::BufferBundle::new(BufferDescriptor {
            label: Some("Mesh Buffer"),
            size: buffer_size_of::<LineMeshData>() * MAX_LINE_MESHES as BufferAddress,
            usage: BufferUsages::STORAGE | BufferUsages::COPY_DST,
            mapped_at_creation: false,
        }))
        .add(BufferLengthComponent::default());
    builder
}

fn line_mesh_instance_buffer_bundle() -> EntityBuilder {
    let mut builder = EntityBuilder::new();
    builder
        .add(LineMeshInstances)
        .add_bundle(antigen_wgpu::BufferBundle::new(BufferDescriptor {
            label: Some("Line Mesh Instance Buffer"),
            size: buffer_size_of::<LineMeshInstanceData>()
                * MAX_LINE_MESH_INSTANCES as BufferAddress,
            usage: BufferUsages::STORAGE | BufferUsages::COPY_DST,
            mapped_at_creation: false,
        }))
        .add(BufferLengthComponent::default());
    builder
}

fn line_instance_buffer_bundle() -> EntityBuilder {
    let mut builder = EntityBuilder::new();
    builder
        .add(LineInstances)
        .add_bundle(antigen_wgpu::BufferBundle::new(BufferDescriptor {
            label: Some("Line Instance Buffer"),
            size: buffer_size_of::<LineInstanceData>() * MAX_LINE_INSTANCES as BufferAddress,
            usage: BufferUsages::VERTEX | BufferUsages::STORAGE | BufferUsages::COPY_DST,
            mapped_at_creation: false,
        }))
        .add(BufferLengthComponent::default());
    builder
}

fn line_vertex_buffer_bundle(entity: Entity, vertices: Vec<LineVertexData>) -> EntityBuilder {
    let mut builder = EntityBuilder::new();
    builder
        .add(LineVertices)
        .add_bundle(antigen_wgpu::BufferBundle::new(BufferDescriptor {
            label: Some("Line Vertex Buffer"),
            size: buffer_size_of::<LineVertexData>() * vertices.len() as BufferAddress,
            usage: BufferUsages::VERTEX | BufferUsages::COPY_DST,
            mapped_at_creation: false,
        }))
        .add_bundle(antigen_wgpu::BufferDataBundle::new(vertices, 0, entity));
    builder
}

fn total_time_builder(uniform_entity: Entity) -> EntityBuilder {
    let mut builder = EntityBuilder::new();
    builder
        .add(StartTimeComponent::construct(Instant::now()))
        .add_bundle(antigen_wgpu::BufferDataBundle::new(
            TotalTimeComponent::construct(0.0),
            buffer_size_of::<[nalgebra::Matrix4<f32>; 2]>()
                + buffer_size_of::<nalgebra::Vector4<f32>>() * 2,
            uniform_entity,
        ));
    builder
}

fn delta_time_bundle(uniform_entity: Entity) -> EntityBuilder {
    let mut builder = EntityBuilder::new();
    builder
        .add(TimestampComponent::construct(Instant::now()))
        .add_bundle(antigen_wgpu::BufferDataBundle::new(
            DeltaTimeComponent::construct(1.0 / 60.0),
            buffer_size_of::<[nalgebra::Matrix4<f32>; 2]>()
                + buffer_size_of::<nalgebra::Vector4<f32>>() * 2
                + buffer_size_of::<f32>(),
            uniform_entity,
        ));
    builder
}

fn perspective_matrix_bundle(uniform_entity: Entity) -> EntityBuilder {
    let mut builder = EntityBuilder::new();
    builder.add(PerspectiveMatrix);
    builder.add_bundle(antigen_wgpu::BufferDataBundle::new(
        PerspectiveMatrixComponent::construct(perspective_matrix(640.0 / 480.0, NEAR_PLANE)),
        0,
        uniform_entity,
    ));
    builder
}

fn orthographic_matrix_bundle(uniform_entity: Entity) -> EntityBuilder {
    let mut builder = EntityBuilder::new();
    builder
        .add(OrthographicMatrix)
        .add_bundle(antigen_wgpu::BufferDataBundle::new(
            OrthographicMatrixComponent::construct(orthographic_matrix(640.0 / 480.0, 200.0)),
            buffer_size_of::<nalgebra::Matrix4<f32>>(),
            uniform_entity,
        ));
    builder
}

fn camera_bundle(uniform_entity: Entity) -> EntityBuilder {
    let mut builder = EntityBuilder::new();
    builder
        .add(Camera)
        .add(EulerAnglesComponent::default())
        .add_bundle(antigen_wgpu::BufferDataBundle::new(
            PositionComponent::construct(Default::default()),
            buffer_size_of::<[nalgebra::Matrix4<f32>; 2]>(),
            uniform_entity,
        ))
        .add_bundle(antigen_wgpu::BufferDataBundle::new(
            RotationComponent::construct(Default::default()),
            buffer_size_of::<[nalgebra::Matrix4<f32>; 2]>()
                + buffer_size_of::<nalgebra::Vector4<f32>>(),
            uniform_entity,
        ));
    builder
}

fn beam_buffer_bundle() -> EntityBuilder {
    let mut builder = EntityBuilder::new();
    builder
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
        ));
    builder
}

fn beam_depth_buffer_bundle() -> EntityBuilder {
    let mut builder = EntityBuilder::new();
    builder
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
        ));
    builder
}

fn beam_multisample_bundle() -> EntityBuilder {
    let mut builder = EntityBuilder::new();
    builder
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
        ));
    builder
}

fn phosphor_buffer_bundle(front: bool) -> EntityBuilder {
    let mut builder = EntityBuilder::new();

    if front {
        builder.add(PhosphorFrontBuffer);
    } else {
        builder.add(PhosphorBackBuffer);
    }

    builder
        .add(BindGroupComponent::default())
        .add_bundle(antigen_wgpu::TextureBundle::new(TextureDescriptor {
            label: Some(if front {
                "Phosphor Front Buffer"
            } else {
                "Phosphor Back Buffer"
            }),
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
                label: Some(if front {
                    "Phosphor Front Buffer View"
                } else {
                    "Phosphor Back Buffer View"
                }),
                format: None,
                dimension: None,
                aspect: TextureAspect::All,
                base_mip_level: 0,
                mip_level_count: None,
                base_array_layer: 0,
                array_layer_count: None,
            },
        ));
    builder
}

fn window_bundle() -> EntityBuilder {
    let mut builder = EntityBuilder::new();
    builder
        .add_bundle(antigen_winit::WindowBundle::default())
        .add_bundle(antigen_winit::WindowTitleBundle::new("Phosphor"))
        .add_bundle(antigen_wgpu::WindowSurfaceBundle::new(
            antigen_wgpu::wgpu::SurfaceConfiguration {
                usage: TextureUsages::RENDER_ATTACHMENT,
                format: TextureFormat::Bgra8UnormSrgb,
                present_mode: antigen_wgpu::wgpu::PresentMode::Fifo,
                width: 0,
                height: 0,
            },
        ))
        .add(RedrawUnconditionally);
    builder
}

// Main assemblage function
pub fn assemble(world: &mut World, channel: &WorldChannel) {
    let window_entity = world.reserve_entity();
    let renderer_entity = world.reserve_entity();

    // Buffer entities
    let uniform_entity = world.spawn(uniform_buffer_bundle().build());

    let vertex_entity = world.spawn(vertex_buffer_bundle().build());
    let triangle_index_entity = world.spawn(triangle_index_buffer_bundle().build());
    let triangle_mesh_entity = world.spawn(triangle_mesh_buffer_bundle().build());
    let triangle_mesh_instance_entity =
        world.spawn(triangle_mesh_instances_buffer_bundle().build());

    let line_vertex_entity = world.reserve_entity();
    world
        .insert(
            line_vertex_entity,
            line_vertex_buffer_bundle(line_vertex_entity, circle_strip(2, 0.5)).build(),
        )
        .unwrap();

    let line_index_entity = world.spawn(line_index_buffer_bundle().build());
    let line_mesh_entity = world.spawn(mesh_buffer_bundle().build());
    let line_mesh_instance_entity = world.spawn(line_mesh_instance_buffer_bundle().build());
    let line_instance_entity = world.spawn(line_instance_buffer_bundle().build());

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

    send_clone_query::<(&LineMeshInstances, &BufferComponent, &BufferLengthComponent), Game>(
        line_mesh_instance_entity,
    )((world, channel))
    .unwrap();

    send_clone_query::<(&LineInstances, &BufferComponent, &BufferLengthComponent), Game>(
        line_instance_entity,
    )((world, channel))
    .unwrap();

    // Time entities
    world.spawn(total_time_builder(uniform_entity).build());
    world.spawn(delta_time_bundle(uniform_entity).build());

    // Camera entities
    world.spawn(perspective_matrix_bundle(uniform_entity).build());
    world.spawn(orthographic_matrix_bundle(uniform_entity).build());
    world.spawn(camera_bundle(uniform_entity).build());

    // Texture entities
    let beam_buffer_entity = world.spawn(beam_buffer_bundle().build());

    let beam_depth_buffer_entity = world.spawn(beam_depth_buffer_bundle().build());

    // Beam multisample resolve target
    let beam_multisample_entity = world.spawn(beam_multisample_bundle().build());

    // Phosphor buffers
    let phosphor_front_entity = world.reserve_entity();
    let phosphor_back_entity = world.reserve_entity();

    world
        .insert(
            phosphor_front_entity,
            phosphor_buffer_bundle(true)
                .add_bundle(
                    antigen_core::swap_with_builder::<TextureViewComponent>(phosphor_back_entity)
                        .build(),
                )
                .add_bundle(
                    antigen_core::swap_with_builder::<BindGroupComponent>(phosphor_back_entity)
                        .build(),
                )
                .build(),
        )
        .unwrap();

    world
        .insert(phosphor_back_entity, phosphor_buffer_bundle(false).build())
        .unwrap();

    // Assemble window
    world
        .insert(window_entity, window_bundle().build())
        .unwrap();

    // Storage bind group
    let storage_bind_group_entity = world.spawn((
        StorageBuffers,
        BindGroupLayoutComponent::default(),
        BindGroupComponent::default(),
    ));

    // Clear pass
    let beam_clear_pass_entity = world.reserve_entity();
    let mut builder = EntityBuilder::new();
    builder.add(BeamClear);
    builder.add(RenderPipelineComponent::default());
    builder.add_bundle(
        antigen_wgpu::RenderPassBundle::draw(
            0,
            Some("Beam Clear".into()),
            vec![(
                beam_multisample_entity,
                Some(beam_buffer_entity),
                Operations {
                    load: LoadOp::Clear(CLEAR_COLOR),
                    store: true,
                },
            )],
            Some((
                beam_depth_buffer_entity,
                Some(Operations {
                    load: LoadOp::Clear(0.0),
                    store: false,
                }),
                None,
            )),
            beam_clear_pass_entity,
            vec![],
            None,
            vec![],
            vec![],
            None,
            None,
            None,
            None,
            (0..1, 0..1),
            renderer_entity,
        )
        .build(),
    );
    world
        .insert(beam_clear_pass_entity, builder.build())
        .unwrap();

    // Beam mesh pass
    let beam_mesh_pass_entity = world.spawn((BeamTriangles, RenderPipelineComponent::default()));

    // Beam line pass
    let beam_line_pass_entity = world.reserve_entity();
    let mut builder = EntityBuilder::new();
    builder.add(BeamLines);
    builder.add(RenderPipelineComponent::default());
    builder.add_bundle(
        antigen_wgpu::RenderPassBundle::draw(
            2,
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
        "test-data/shaders/beam.wgsl",
    );

    // Phosphor pass
    let phosphor_pass_entity = world.reserve_entity();
    let mut builder = EntityBuilder::new();
    builder.add(PhosphorDecay);
    builder.add(RenderPipelineComponent::default());
    builder.add(BindGroupLayoutComponent::default());
    builder.add_bundle(
        antigen_wgpu::RenderPassBundle::draw(
            3,
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
        "test-data/shaders/phosphor_decay.wgsl",
    );

    // Tonemap pass
    let tonemap_pass_entity = world.reserve_entity();

    let mut builder = EntityBuilder::new();
    builder.add(Tonemap);
    builder.add(RenderPipelineComponent::default());
    builder.add_bundle(
        antigen_wgpu::RenderPassBundle::draw(
            4,
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
        "test-data/shaders/tonemap.wgsl",
    );

    // Renderer
    let mut builder = EntityBuilder::new();

    builder.add(PhosphorRenderer);

    builder.add(PlayerInputComponent::default());

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

    // Insert tagged entities
    insert_tagged_entity::<Uniform>(world, uniform_entity);
    insert_tagged_entity::<BeamBuffer>(world, beam_buffer_entity);
    insert_tagged_entity::<BeamDepthBuffer>(world, beam_depth_buffer_entity);
    insert_tagged_entity::<BeamMultisample>(world, beam_multisample_entity);
    insert_tagged_entity::<StorageBuffers>(world, storage_bind_group_entity);
    insert_tagged_entity::<BeamTriangles>(world, beam_mesh_pass_entity);
    insert_tagged_entity::<PhosphorRenderer>(world, renderer_entity);

    insert_tagged_entity::<Vertices>(world, vertex_entity);
    insert_tagged_entity::<TriangleIndices>(world, triangle_index_entity);
    insert_tagged_entity::<TriangleMeshes>(world, triangle_mesh_entity);
    insert_tagged_entity::<TriangleMeshInstances>(world, triangle_mesh_instance_entity);
    insert_tagged_entity::<LineIndices>(world, line_index_entity);
    insert_tagged_entity::<LineMeshes>(world, line_mesh_entity);
    insert_tagged_entity::<LineMeshInstances>(world, line_mesh_instance_entity);
    insert_tagged_entity::<LineInstances>(world, line_instance_entity);

    // Insert tagged entities on game thread
    channel
        .send_to::<Game>(insert_tagged_entity_by_query_message::<
            (&TriangleMeshInstances, &BufferComponent),
            TriangleMeshInstances,
        >())
        .unwrap();

    channel
        .send_to::<Game>(insert_tagged_entity_by_query_message::<
            (&LineMeshInstances, &BufferComponent),
            LineMeshInstances,
        >())
        .unwrap();

    channel
        .send_to::<Game>(insert_tagged_entity_by_query_message::<
            (&LineInstances, &BufferComponent),
            LineInstances,
        >())
        .unwrap();

    // Load SVG meshes
    {
        let svg = SvgLayers::parse("test-data/fonts/basic.svg")
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
                register_line_mesh_id(world, key.into(), (line_mesh, line_count));

                let mut builder = line_mesh_builder(world, vertices, indices);
                let bundle = builder.build();

                world.spawn(bundle);
            }
        }
    }

    assemble_test_geometry(world);

    load_map::<MapFile, Filesystem, _>(
        channel,
        //"test-data/maps/non_manifold_line.map",
        //"test-data/maps/non_manifold_room.map",
        "test-data/maps/line_index_test.map",
    );
}

fn assemble_test_geometry(world: &mut World) {
    let line_mesh_entity = get_tagged_entity::<LineMeshes>(world).unwrap();

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

    register_line_mesh_id(
        world,
        "triangle_equilateral".into(),
        (line_mesh, line_count),
    );

    let mut builder = line_strip_mesh_builder(world, vertices);
    let bundle = builder.build();
    world.spawn(bundle);
}

#[derive(Clone)]
struct MapData {
    geo_map: antigen_shambler::shambler::GeoMap,
    lines: antigen_shambler::shambler::line::Lines,
    entity_centers: antigen_shambler::shambler::entity::EntityCenters,
    brush_centers: antigen_shambler::shambler::brush::BrushCenters,
    face_vertices: antigen_shambler::shambler::face::FaceVertices,
    face_duplicates: antigen_shambler::shambler::face::FaceDuplicates,
    face_triangle_indices: antigen_shambler::shambler::face::FaceTriangleIndices,
    face_lines: antigen_shambler::shambler::face::FaceLines,
    interior_faces: antigen_shambler::shambler::face::InteriorFaces,
    face_face_containment: antigen_shambler::shambler::face::FaceFaceContainment,
    brush_face_containment: antigen_shambler::shambler::brush::BrushFaceContainment,
    manifold_lines: antigen_shambler::shambler::line::ManifoldLines,
    non_manifold_lines: antigen_shambler::shambler::line::NonManifoldLines,
}

impl From<GeoMap> for MapData {
    fn from(geo_map: GeoMap) -> Self {
        let face_brushes = antigen_shambler::shambler::face::face_brushes(&geo_map.brush_faces);
        let brush_entities =
            antigen_shambler::shambler::brush::brush_entities(&geo_map.entity_brushes);

        // Create geo planes from brush planes
        let face_planes = antigen_shambler::shambler::face::face_planes(&geo_map.face_planes);

        // Create per-brush hulls from brush planes
        let brush_hulls =
            antigen_shambler::shambler::brush::brush_hulls(&geo_map.brush_faces, &face_planes);

        // Generate face vertices
        let (face_vertices, _) = antigen_shambler::shambler::face::face_vertices(
            &geo_map.brush_faces,
            &face_planes,
            &brush_hulls,
        );

        let face_normals =
            antigen_shambler::shambler::face::normals_flat(&face_vertices, &face_planes);

        // Find duplicate faces
        let face_duplicates = antigen_shambler::shambler::face::face_duplicates(
            &geo_map.faces,
            &face_planes,
            &face_vertices,
        );

        // Generate centers
        let face_centers = antigen_shambler::shambler::face::face_centers(&face_vertices);

        let brush_centers =
            antigen_shambler::shambler::brush::brush_centers(&geo_map.brush_faces, &face_centers);

        let entity_centers = antigen_shambler::shambler::entity::entity_centers(
            &geo_map.entity_brushes,
            &brush_centers,
        );

        // Generate per-plane CCW face indices
        let face_indices = antigen_shambler::shambler::face::face_indices(
            &geo_map.face_planes,
            &face_planes,
            &face_vertices,
            &face_centers,
            antigen_shambler::shambler::face::FaceWinding::Clockwise,
        );

        let face_triangle_indices =
            antigen_shambler::shambler::face::face_triangle_indices(&face_indices);

        let (lines, face_lines) = antigen_shambler::shambler::line::lines(&face_indices);

        // Generate tangents
        let face_bases = antigen_shambler::shambler::face::face_bases(
            &geo_map.faces,
            &face_planes,
            &geo_map.face_offsets,
            &geo_map.face_angles,
            &geo_map.face_scales,
        );

        // Calculate face-face containment
        let face_face_containment = antigen_shambler::shambler::face::face_face_containment(
            &geo_map.faces,
            &lines,
            &face_planes,
            &face_bases,
            &face_vertices,
            &face_lines,
        );

        // Calculate brush-face containment
        let mut brush_face_containment = antigen_shambler::shambler::brush::brush_face_containment(
            &geo_map.brushes,
            &geo_map.faces,
            &geo_map.brush_faces,
            &brush_hulls,
            &face_vertices,
        );

        // Remove non-culled faced from brush-face containment
        // TODO: This is a hack for the sake of omitting brushes from the culling process.
        //       It should be replaced with a more robust filtering system.
        for brush in brush_face_containment.keys().copied().collect::<Vec<_>>() {
            let entity = &brush_entities[&brush];
            let properties = &geo_map.entity_properties[entity];
            if matches!(
                Self::property_usize("mesh.visual.cull.faces", properties),
                Err(_)
            ) {
                brush_face_containment.remove(&brush);
            }
        }

        // Line-face connections
        let line_faces = antigen_shambler::shambler::line::line_faces(&face_lines);

        let mut culled_lines = lines.clone();
        for line in culled_lines.keys().copied().collect::<Vec<_>>() {
            let face = &line_faces[&line];
            if brush_face_containment
                .iter()
                .any(|(_, rhs)| rhs.contains(face))
            {
                culled_lines.remove(&line);
            }
        }

        let line_face_connections = antigen_shambler::shambler::line::line_face_connections(
            &culled_lines,
            &line_faces,
            &face_vertices,
        );
        let (manifold_lines, non_manifold_lines) =
            antigen_shambler::shambler::line::manifold_lines(&line_face_connections);

        let interior_faces = antigen_shambler::shambler::face::interior_faces(
            &geo_map.faces,
            &face_lines,
            &face_normals,
            &face_centers,
            &non_manifold_lines,
            &line_face_connections,
        );

        MapData {
            geo_map,
            lines,
            entity_centers,
            brush_centers,
            face_vertices,
            face_duplicates,
            face_triangle_indices,
            face_lines,
            interior_faces,
            face_face_containment,
            brush_face_containment,
            manifold_lines,
            non_manifold_lines,
        }
    }
}

impl MapData {
    fn default_entity_name(entity: &EntityId) -> String {
        format!("entity_{entity:}")
    }

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
            position: [v.x * scale_factor, v.z * scale_factor, -v.y * scale_factor],
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

    fn assemble_brush_entity_triangle_mesh(
        &self,
        entity: &EntityId,
        cull_face: impl Fn(&FaceId) -> bool,
        cull_line: impl Fn(&LineId) -> bool,
    ) -> (Vec<VertexData>, Vec<TriangleIndexData>, Vec<LineIndexData>) {
        let brushes = &self.geo_map.entity_brushes[entity];

        let mut mesh_vertices: Vec<VertexData> = Default::default();
        let mut triangle_indices: Vec<TriangleIndexData> = Default::default();
        let mut line_indices: Vec<LineIndexData> = Default::default();

        let mut local_vertex_head = 0u16;
        let mut local_index_head = 0u32;

        let entity_faces = self.entity_faces(brushes);
        let entity_center = self.entity_centers[entity];

        for face_id in entity_faces.filter(|face_id| cull_face(face_id)) {
            // Fetch and interpret texture data
            let texture_name = self.face_texture(&face_id);
            let color = Self::face_color(texture_name);
            let intensity = Self::face_intensity(texture_name);

            let verts = self
                .face_vertices(face_id, color, intensity, 1.0)
                .map(|vertex| VertexData {
                    position: [
                        vertex.position[0] - entity_center[0],
                        vertex.position[1] - entity_center[2],
                        vertex.position[2] - -entity_center[1],
                    ],
                    ..vertex
                })
                .collect::<Vec<_>>();
            let vertex_count = verts.len();
            mesh_vertices.extend(verts);

            let face_triangle_indices = &self.face_triangle_indices[&face_id];
            triangle_indices.extend(
                face_triangle_indices
                    .iter()
                    .map(move |i| *i as u16 + local_vertex_head),
            );

            let face_lines = &self.face_lines[&face_id];
            line_indices.extend(face_lines.iter().filter(|line| cull_line(line)).flat_map(
                move |line_id| {
                    let antigen_shambler::shambler::line::Line { i0, i1 } = self.lines[line_id];
                    [
                        (i0 + local_index_head as usize) as u32,
                        (i1 + local_index_head as usize) as u32,
                    ]
                },
            ));

            local_vertex_head += vertex_count as u16;
            local_index_head += vertex_count as u32;
        }

        (mesh_vertices, triangle_indices, line_indices)
    }

    pub fn assemble_brush_entities_render_thread(&self, world: &mut World) -> Vec<EntityBuilder> {
        let entity_brushes = self.classname_brushes("brush");
        let mut builders = vec![];

        // Brush entity meshes
        for (entity, _) in entity_brushes {
            let properties = self.geo_map.entity_properties.get(entity).unwrap();

            if matches!(Self::property_bool("mesh.visual", properties), Ok(true)) {
                let entity_mesh_name = Self::property_targetname("mesh.visual.name", properties)
                    .unwrap_or_else(|_| Self::default_entity_name(entity));

                // Generate mesh
                let (mesh_vertices, triangle_indices, line_indices) = self
                    .assemble_brush_entity_triangle_mesh(
                        entity,
                        self.face_cull_predicate(entity, "mesh.visual"),
                        self.line_cull_predicate(entity, "mesh.visual"),
                    );

                let ty = Self::property_usize("mesh.visual.type", properties)
                    .expect("No mesh.visual.type property");
                builders.extend(match ty {
                    1 => Self::build_brush_entity_triangle_meshes(
                        world,
                        &entity_mesh_name,
                        mesh_vertices,
                        triangle_indices,
                    ),
                    2 => Self::build_brush_entity_line_meshes(
                        world,
                        &entity_mesh_name,
                        mesh_vertices,
                        line_indices,
                    ),
                    3 => Self::build_brush_entity_triangle_line_meshes(
                        world,
                        &entity_mesh_name,
                        mesh_vertices,
                        triangle_indices,
                        line_indices,
                    ),
                    _ => unimplemented!(),
                });
            }
        }

        builders
    }

    fn entity_line(world: &mut World, entity: &EntityId, properties: &Properties) -> EntityBuilder {
        let mut builder = EntityBuilder::new();
        if matches!(Self::property_bool("line", properties), Ok(true)) {
            let name = Self::property_string("line.name", properties)
                .map(ToString::to_string)
                .unwrap_or_else(|_| Self::default_entity_name(entity));

            let line_count = Self::property_usize("line.segments", properties).unwrap_or(1);
            let color =
                Self::property_f32_3("line.color", properties).unwrap_or_else(|_| (1.0, 1.0, 1.0));
            let intensity = Self::property_f32("line.intensity", properties).unwrap_or(1.0);
            let delta_intensity =
                Self::property_f32("line.delta_intensity", properties).unwrap_or(1.0);

            builder.add_bundle(
                line_builder(
                    world,
                    name.into(),
                    line_count,
                    color,
                    intensity,
                    delta_intensity,
                )
                .build(),
            );
        }
        builder
    }

    fn entity_oscilloscope(properties: &Properties) -> EntityBuilder {
        let mut builder = EntityBuilder::new();
        if matches!(Self::property_bool("oscilloscope", properties), Ok(true)) {
            let speed = Self::property_f32("oscilloscope.speed", properties).unwrap_or(1.0);
            let magnitude = Self::property_f32("oscilloscope.magnitude", properties).unwrap_or(1.0);

            let x = Self::property_expression_f32("oscilloscope.x", properties)
                .unwrap_or(Expression::Val(0.0));
            let y = Self::property_expression_f32("oscilloscope.y", properties)
                .unwrap_or(Expression::Val(0.0));
            let z = Self::property_expression_f32("oscilloscope.z", properties)
                .unwrap_or(Expression::Val(0.0));

            builder.add(Oscilloscope::new(speed, magnitude, move |f| {
                let vars = [("f", f)].into_iter().collect::<BTreeMap<_, _>>();
                (x.eval(&vars), y.eval(&vars), z.eval(&vars))
            }));
        }
        builder
    }

    pub fn assemble_point_entities_render_thread(&self, world: &mut World) -> Vec<EntityBuilder> {
        let mut builders = vec![];

        for entity in self.geo_map.point_entities.iter() {
            let mut builder = EntityBuilder::new();

            let properties = self.geo_map.entity_properties.get(entity).unwrap();

            builder.add_bundle(Self::entity_line(world, entity, properties).build());
            builder.add_bundle(Self::entity_oscilloscope(properties).build());

            builders.push(builder);
        }

        builders
    }

    pub fn face_cull_predicate(
        &self,
        entity: &EntityId,
        component_property: &str,
    ) -> impl Fn(&FaceId) -> bool + '_ {
        let properties = &self.geo_map.entity_properties[entity];
        let component_property = component_property.to_string();
        move |face_id| {
            if let Ok(cull) =
                Self::property_usize(&(component_property.clone() + ".cull.faces"), properties)
            {
                if cull & 1 > 0 && self.face_duplicates.iter().any(|(_, b)| b == face_id) {
                    return false;
                }

                if cull & 2 > 0
                    && self
                        .face_face_containment
                        .iter()
                        .any(|(_, b)| b.contains(face_id))
                {
                    return false;
                }

                if cull & 4 > 0
                    && self
                        .brush_face_containment
                        .iter()
                        .any(|(_, b)| b.contains(face_id))
                {
                    return false;
                }

                if cull & 8 > 0 && !self.interior_faces.contains(&face_id) {
                    return false;
                }

                if cull & 16 > 0 && self.interior_faces.contains(&face_id) {
                    return false;
                }
            }

            true
        }
    }

    pub fn line_cull_predicate(
        &self,
        entity: &EntityId,
        component_property: &str,
    ) -> impl Fn(&LineId) -> bool + '_ {
        let properties = &self.geo_map.entity_properties[entity];
        let component_property = component_property.to_string();
        move |line_id| {
            if let Ok(cull) =
                Self::property_usize(&(component_property.clone() + ".cull.lines"), properties)
            {
                if cull & 1 > 0 && self.manifold_lines.iter().any(|id| id == line_id) {
                    return false;
                }

                if cull & 2 > 0 && self.non_manifold_lines.iter().any(|id| id == line_id) {
                    return false;
                }
            }

            true
        }
    }

    pub fn assemble_brush_entities_game_thread(&self, world: &mut World) {
        let (_, shared_shapes) = world
            .query_mut::<&mut SharedShapesComponent>()
            .into_iter()
            .next()
            .expect("No SharedShapesComponent");

        for (entity, brushes) in self.classname_brushes("brush") {
            let properties = &self.geo_map.entity_properties[entity];

            let entity_center = self.entity_centers[entity];

            if matches!(Self::property_bool("convex_hull", properties), Ok(true)) {
                let key = Self::property_targetname("convex_hull.name", properties)
                    .unwrap_or_else(|_| Self::default_entity_name(entity));

                let shape = match Self::property_string("convex_hull.type", properties).unwrap() {
                    "single" => {
                        let mut brush_vertices = vec![];
                        for brush in brushes {
                            for face in &self.geo_map.brush_faces[brush] {
                                let face_vertices = &self.face_vertices[face];
                                for vertex in face_vertices {
                                    if !brush_vertices.contains(vertex) {
                                        brush_vertices.push(*vertex);
                                    }
                                }
                            }
                        }

                        let brush_vertices = brush_vertices
                            .into_iter()
                            .map(|vertex| vertex.xzy() - entity_center.xzy())
                            .collect::<Vec<_>>();

                        vec![(
                            rapier3d::prelude::nalgebra::Isometry::identity(),
                            brush_vertices,
                        )]
                    }
                    "compound" => {
                        let mut brush_hulls = vec![];

                        for brush in brushes {
                            let brush_center = self.brush_centers[brush];
                            let mut brush_vertices = vec![];
                            for face in &self.geo_map.brush_faces[brush] {
                                let face_vertices = &self.face_vertices[face];
                                for vertex in face_vertices {
                                    if !brush_vertices.contains(vertex) {
                                        brush_vertices.push(vertex.xzy() - brush_center.xzy());
                                    }
                                }
                            }

                            brush_hulls.push((
                                rapier3d::prelude::Isometry::new(
                                    rapier3d::prelude::nalgebra::Vector3::<f32>::new(
                                        brush_center.x - entity_center.x,
                                        brush_center.z - entity_center.z,
                                        brush_center.y - entity_center.y,
                                    ),
                                    rapier3d::prelude::nalgebra::Vector3::<f32>::zeros(),
                                ),
                                brush_vertices,
                            ));
                        }

                        brush_hulls
                    }
                    _ => unimplemented!(),
                };

                let shape_fn = move |scale: nalgebra::Vector3<f32>| {
                    let mut compound = vec![];
                    for (mut isometry, convex_hull) in &shape {
                        isometry.translation.x *= scale.x;
                        isometry.translation.y *= scale.y;
                        isometry.translation.z *= scale.z;

                        let mut scaled_hull = vec![];
                        for vertex in convex_hull {
                            let scaled_vert = nalgebra::vector![
                                vertex.x * scale.x,
                                vertex.y * scale.y,
                                vertex.z * scale.z
                            ];
                            scaled_hull.push(rapier3d::prelude::nalgebra::Point3::new(
                                scaled_vert.x,
                                scaled_vert.y,
                                scaled_vert.z,
                            ));
                        }
                        compound.push((
                            isometry,
                            SharedShape::convex_hull(&scaled_hull[..]).unwrap(),
                        ))
                    }
                    if compound.len() == 1 {
                        compound.remove(0).1
                    } else {
                        SharedShape::compound(compound)
                    }
                };

                shared_shapes.insert(key.to_owned(), Box::new(shape_fn));
            }

            if matches!(Self::property_bool("mesh.collision", properties), Ok(true)) {
                let key = Self::property_targetname("mesh.collision.name", properties)
                    .unwrap_or_else(|_| Self::default_entity_name(entity));

                let (mesh_vertices, triangle_indices, _) = self
                    .assemble_brush_entity_triangle_mesh(
                        entity,
                        self.face_cull_predicate(entity, "mesh.collision"),
                        |_| false,
                    );

                let mesh_vertices = mesh_vertices
                    .into_iter()
                    .map(|VertexData { position, .. }| {
                        rapier3d::prelude::nalgebra::Point3::new(
                            position[0],
                            position[1],
                            position[2],
                        )
                    })
                    .collect::<Vec<_>>();

                let triangle_indices = triangle_indices
                    .chunks(3)
                    .map(|inds| [inds[0] as u32, inds[1] as u32, inds[2] as u32])
                    .collect::<Vec<_>>();

                let shape_fn =
                    move |_| SharedShape::trimesh(mesh_vertices.clone(), triangle_indices.clone());

                shared_shapes.insert(key, Box::new(shape_fn));
            }
        }
    }

    fn build_brush_entity_triangle_line_meshes(
        world: &mut World,
        entity_mesh_name: &str,
        vertices: Vec<VertexData>,
        triangle_indices: Vec<TriangleIndexData>,
        line_indices: Vec<LineIndexData>,
    ) -> Vec<EntityBuilder> {
        let mut builders = vec![];

        // Vertex information
        let vertex_entity = get_tagged_entity::<Vertices>(world).unwrap();
        let triangle_index_entity = get_tagged_entity::<TriangleIndices>(world).unwrap();
        let triangle_mesh_entity = get_tagged_entity::<TriangleMeshes>(world).unwrap();
        let line_index_entity = get_tagged_entity::<LineIndices>(world).unwrap();
        let line_mesh_entity = get_tagged_entity::<LineMeshes>(world).unwrap();

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

        let vertex_count = vertices.len() as u32;
        let triangle_index_count = triangle_indices.len() as u32;
        let line_index_count = line_indices.len() as u32;

        builders.extend([
            triangle_mesh_builder(world, vertices, triangle_indices),
            triangle_mesh_data_builder(
                world,
                triangle_index_count,
                0,
                base_triangle_index,
                base_vertex,
            ),
        ]);

        builders.extend([
            line_indices_builder(world, line_indices),
            line_mesh_data_builder(
                world,
                base_vertex,
                vertex_count,
                base_line_index,
                line_index_count,
            ),
        ]);

        register_triangle_mesh_id(world, entity_mesh_name.to_owned().into(), triangle_mesh);
        register_line_mesh_id(
            world,
            entity_mesh_name.to_owned().into(),
            (line_mesh as u32, line_index_count / 2),
        );

        builders
    }

    fn build_brush_entity_triangle_meshes(
        world: &mut World,
        entity_mesh_name: &str,
        vertices: Vec<VertexData>,
        triangle_indices: Vec<TriangleIndexData>,
    ) -> Vec<EntityBuilder> {
        let mut builders = vec![];

        // Vertex information
        let vertex_entity = get_tagged_entity::<Vertices>(world).unwrap();
        let triangle_index_entity = get_tagged_entity::<TriangleIndices>(world).unwrap();
        let triangle_mesh_entity = get_tagged_entity::<TriangleMeshes>(world).unwrap();

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

        let triangle_index_count = triangle_indices.len() as u32;

        builders.extend([
            triangle_mesh_builder(world, vertices, triangle_indices),
            triangle_mesh_data_builder(
                world,
                triangle_index_count,
                0,
                base_triangle_index,
                base_vertex,
            ),
        ]);

        register_triangle_mesh_id(world, entity_mesh_name.to_owned().into(), triangle_mesh);

        builders
    }

    fn build_brush_entity_line_meshes(
        world: &mut World,
        entity_mesh_name: &str,
        vertices: Vec<VertexData>,
        line_indices: Vec<LineIndexData>,
    ) -> Vec<EntityBuilder> {
        let mut builders = vec![];

        // Vertex information
        let vertex_entity = get_tagged_entity::<Vertices>(world).unwrap();
        let line_index_entity = get_tagged_entity::<LineIndices>(world).unwrap();
        let line_mesh_entity = get_tagged_entity::<LineMeshes>(world).unwrap();

        let base_vertex = world
            .query_one_mut::<&BufferLengthComponent>(vertex_entity)
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

        let vertex_count = vertices.len() as u32;
        let line_index_count = line_indices.len() as u32;

        builders.extend([
            line_mesh_builder(world, vertices, line_indices),
            line_mesh_data_builder(
                world,
                base_vertex,
                vertex_count,
                base_line_index,
                line_index_count,
            ),
        ]);

        register_line_mesh_id(
            world,
            entity_mesh_name.to_owned().into(),
            (line_mesh as u32, line_index_count / 2),
        );

        builders
    }

    fn property_origin(properties: &Properties) -> Option<nalgebra::Vector3<f32>> {
        Self::property_f32_3("origin", properties)
            .ok()
            .map(|(x, z, y)| nalgebra::vector![x, y, -z])
    }

    fn property_rotation(properties: &Properties, convert: bool) -> nalgebra::UnitQuaternion<f32> {
        let y_ofs = if convert { 90.0f32.to_radians() } else { 0.0 };
        if let Ok((x, y, z)) = Self::property_f32_3("mangle", properties) {
            nalgebra::UnitQuaternion::from_euler_angles(
                z.to_radians(),
                y.to_radians() + y_ofs,
                -x.to_radians(),
            )
        } else if let Ok(y) = Self::property_f32("angle", properties) {
            nalgebra::UnitQuaternion::from_euler_angles(0.0, y.to_radians() + y_ofs, 0.0)
        } else {
            nalgebra::UnitQuaternion::default()
        }
    }

    fn property_scale(properties: &Properties) -> nalgebra::Vector3<f32> {
        if let Ok((x, y, z)) = Self::property_f32_3("scale", properties) {
            nalgebra::vector![x, z, y]
        } else {
            nalgebra::vector![1.0, 1.0, 1.0]
        }
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

    fn property_usize(key: &str, properties: &Properties) -> Result<usize, Box<dyn Error>> {
        Ok(properties
            .0
            .iter()
            .find(|p| p.key == key)
            .ok_or("Key not found")?
            .value
            .parse::<usize>()?)
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

    fn property_bool<'a>(key: &str, properties: &'a Properties) -> Result<bool, Box<dyn Error>> {
        Ok(
            match properties
                .0
                .iter()
                .find(|p| p.key == key)
                .ok_or("Key not found")?
                .value
                .as_str()
            {
                "true" => true,
                "false" => false,
                _ => panic!("Incorrect variant for property {key:}"),
            },
        )
    }

    fn property_target(property: &str, properties: &Properties) -> Result<String, Box<dyn Error>> {
        if let Ok(mesh) = Self::property_string(property, properties) {
            Ok(mesh.to_owned())
        } else if matches!(
            Self::property_bool(&format!("{property}.use_target"), properties),
            Ok(true)
        ) {
            Ok(Self::property_string("target", properties)?.to_owned())
        } else {
            Err("No such property".into())
        }
    }

    fn property_targetname(
        property: &str,
        properties: &Properties,
    ) -> Result<String, Box<dyn Error>> {
        if let Ok(mesh) = Self::property_string(property, properties) {
            Ok(mesh.to_owned())
        } else if matches!(
            Self::property_bool(&format!("{property}.use_targetname"), properties),
            Ok(true)
        ) {
            Ok(Self::property_string("targetname", properties)?.to_owned())
        } else {
            Err("No such property".into())
        }
    }

    fn entity_line_mesh_instance(entity: &EntityId, properties: &Properties) -> EntityBuilder {
        let mut builder = EntityBuilder::new();
        if let Ok(true) = MapData::property_bool("mesh_instance.line", properties) {
            let mesh = MapData::property_target("mesh_instance.line.mesh", properties)
                .unwrap_or_else(|_| Self::default_entity_name(entity));
            builder.add(LineMeshInstanceComponent::construct(Cow::Owned(mesh)));
        }
        builder
    }

    fn entity_triangle_mesh_instance(entity: &EntityId, properties: &Properties) -> EntityBuilder {
        let mut builder = EntityBuilder::new();
        if let Ok(true) = Self::property_bool("mesh_instance.triangle", properties) {
            let mesh = Self::property_target("mesh_instance.triangle.mesh", properties)
                .unwrap_or_else(|_| Self::default_entity_name(entity));
            builder.add(TriangleMeshInstanceComponent::construct(Cow::Owned(mesh)));
        }
        builder
    }

    fn entity_rigid_body(properties: &Properties) -> EntityBuilder {
        let mut builder = EntityBuilder::new();
        if let Ok(true) = Self::property_bool("rigid_body", properties) {
            if let Ok(ty) = Self::property_string("rigid_body.type", properties) {
                let rigid_body_builder = match ty {
                    "dynamic" => RigidBodyBuilder::new_dynamic(),
                    "kinematic_position_based" => RigidBodyBuilder::new_kinematic_position_based(),
                    "kinematic_velocity_based" => RigidBodyBuilder::new_kinematic_velocity_based(),
                    "static" => RigidBodyBuilder::new_static(),
                    _ => panic!("Incorrect variant for rigid_body.type"),
                };
                builder.add(RigidBodyComponent::construct(rigid_body_builder.build()));
            }

            if let Ok(vel) = Self::property_f32_3("rigid_body.linear_velocity", properties) {
                builder.add(LinearVelocityComponent::construct(nalgebra::vector![
                    vel.0, vel.1, vel.2
                ]));
            }

            if let Ok(vel) = Self::property_f32_3("rigid_body.angular_velocity", properties) {
                builder.add(AngularVelocityComponent::construct(nalgebra::vector![
                    vel.0, vel.1, vel.2
                ]));
            }
        }
        builder
    }

    fn entity_collider(
        world: &mut World,
        entity: &EntityId,
        properties: &Properties,
        scale: nalgebra::Vector3<f32>,
    ) -> EntityBuilder {
        let mut builder = EntityBuilder::new();
        if let Ok(true) = Self::property_bool("collider", properties) {
            if let Ok(shape) = Self::property_string("collider.shape", properties) {
                let collider_builder = match shape {
                    "ball" => {
                        let radius =
                            Self::property_f32("collider.ball.radius", properties).unwrap();
                        ColliderBuilder::ball(radius * scale.x.max(scale.y).max(scale.z))
                    }
                    "cuboid" => {
                        let extents =
                            Self::property_f32_3("collider.cuboid.extents", properties).unwrap();
                        ColliderBuilder::cuboid(
                            extents.0 * scale.x,
                            extents.1 * scale.y,
                            extents.2 * scale.z,
                        )
                    }
                    "convex_hull" => {
                        let mesh = Self::property_target("collider.convex_hull.mesh", properties)
                            .unwrap_or_else(|_| Self::default_entity_name(entity));

                        let (_, shared_shapes) = world
                            .query_mut::<&SharedShapesComponent>()
                            .into_iter()
                            .next()
                            .expect("No SharedShapesComponent");

                        let shape = shared_shapes[&mesh](scale);

                        ColliderBuilder::new(shape)
                    }
                    "trimesh" => {
                        let mesh = Self::property_target("collider.trimesh.mesh", properties)
                            .unwrap_or_else(|_| Self::default_entity_name(entity));

                        let (_, shared_shapes) = world
                            .query_mut::<&SharedShapesComponent>()
                            .into_iter()
                            .next()
                            .expect("No SharedShapesComponent");

                        let shape = shared_shapes[&mesh](scale);

                        ColliderBuilder::new(shape)
                    }
                    _ => panic!("Incorrect variant for collider.shape"),
                };

                let collider_builder = if let Ok(restitution) =
                    Self::property_f32("collider.restitution", properties)
                {
                    collider_builder.restitution(restitution)
                } else {
                    collider_builder
                };

                let collider_builder =
                    if let Ok(ty) = Self::property_string("collider.type", properties) {
                        match ty {
                            "solid" => collider_builder,
                            "sensor" => collider_builder.sensor(true),
                            _ => unimplemented!(),
                        }
                    } else {
                        collider_builder
                    };

                let collider_builder = if let Ok(active_events) =
                    Self::property_usize("collider.events.active", properties)
                {
                    let mut ae = ActiveEvents::default();

                    if active_events & 1 > 0 {
                        ae |= ActiveEvents::CONTACT_EVENTS;
                    }

                    if active_events & 2 > 0 {
                        ae |= ActiveEvents::INTERSECTION_EVENTS;
                    }

                    if active_events > 0 {
                        builder.add(ColliderEventOutputComponent::construct(Default::default()));

                        let target = Self::property_target("collider.events.target", properties);

                        if let Ok(target) = target {
                            builder.add(EventTargetComponent::<IntersectionEvent>::construct(
                                target.to_owned().into(),
                            ));
                        }
                    }

                    collider_builder.active_events(ae)
                } else {
                    collider_builder
                };

                builder.add(ColliderComponent::construct(collider_builder.build()));
            }
        }
        builder
    }

    fn entity_mover(properties: &Properties) -> EntityBuilder {
        let mut builder = EntityBuilder::new();
        if let Ok(true) = Self::property_bool("mover", properties) {
            if let Ok((x, y, z)) = Self::property_f32_3("mover.offset.position", properties) {
                builder.add(PositionOffsetComponent::construct((
                    nalgebra::vector![x, z, y],
                    nalgebra::vector![0.0, 0.0, 0.0],
                )));
            }

            if let Ok((x, y, z)) = Self::property_f32_3("mover.offset.rotation", properties) {
                builder.add(RotationOffsetComponent::construct((
                    nalgebra::vector![x, z, y],
                    nalgebra::vector![0.0, 0.0, 0.0],
                )));
            }

            if let Ok(speed) = Self::property_f32("mover.speed", properties) {
                builder.add(SpeedComponent::construct(speed));
            }

            if let Ok(open) = Self::property_bool("mover.open", properties) {
                builder.add(MoverOpenComponent::construct(open));
            }

            if let Ok(true) = Self::property_bool("mover.events", properties) {
                let name =
                    Self::property_targetname("mover.name", properties).expect("Mover has no name");
                builder.add(NamedEntityComponent::construct(name.to_owned().into()));

                builder.add(MoverEventInputComponent::construct(Default::default()));
            }
        }
        builder
    }

    fn entity_event(properties: &Properties) -> EntityBuilder {
        let mut builder = EntityBuilder::new();
        if let Ok(true) = Self::property_bool("event", properties) {
            let transform = EventTransformComponent::unit();

            let input = Self::property_string("event.in", properties).unwrap();
            let transform = match input {
                "collider.intersection.enter" | "collider.intersection.exit" => {
                    builder.add(ColliderEventInputComponent::construct(Default::default()));
                    transform.with_input_type::<IntersectionEvent>()
                }
                _ => unimplemented!(),
            };

            let output = Self::property_string("event.out", properties).unwrap();
            let transform = match output {
                "mover.open" => {
                    builder.add(MoverEventOutputComponent::construct(Default::default()));
                    transform.with_output_type::<MoverEvent>()
                }
                _ => unimplemented!(),
            };

            builder.add(transform);

            let target =
                Self::property_target("event.target", properties).expect("Event has no target");
            builder.add(EventTargetComponent::<MoverEvent>::construct(
                target.to_owned().into(),
            ));

            let name =
                Self::property_targetname("event.name", properties).expect("Event has no name");
            builder.add(NamedEntityComponent::construct(name.to_owned().into()));
        }
        builder
    }

    fn entity_text(
        properties: &Properties,
        origin: nalgebra::Vector3<f32>,
        scale: nalgebra::Vector3<f32>,
    ) -> Vec<EntityBuilder> {
        let mut builders = vec![];
        if let Ok(true) = Self::property_bool("text", properties) {
            let string = Self::property_string("text.string", properties).unwrap();
            let rotation = Self::property_rotation(properties, true);

            let lines = string
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

                    let mut builder = EntityBuilder::new();
                    builder.add(PositionComponent::construct(origin + ofs));
                    builder.add(RotationComponent::construct(rotation));
                    builder.add(ScaleComponent::construct(scale));
                    builder.add(LineMeshInstanceComponent::construct(Cow::Owned(key)));
                    builders.push(builder);
                }
            }
        }
        builders
    }

    pub fn assemble_entities_game_thread(&self, world: &mut World) -> Vec<EntityBuilder> {
        let mut builders: Vec<EntityBuilder> = vec![];

        // Spawn generic point entities
        let entities = self.geo_map.entities.iter().flat_map(|entity| {
            let properties = self.geo_map.entity_properties.get(entity)?;
            if let Some(classname) = properties.0.iter().find(|p| p.key == "classname") {
                if classname.value == "point" || classname.value == "brush" {
                    Some((entity, properties))
                } else {
                    None
                }
            } else {
                None
            }
        });

        for (entity, properties) in entities.into_iter() {
            let origin = Self::property_origin(properties).unwrap_or_else(|| {
                self.entity_centers
                    .get(entity)
                    .map(|center| nalgebra::vector![center.x, center.z, -center.y])
                    .unwrap_or(nalgebra::Vector3::zeros())
            });
            let rotation = Self::property_rotation(properties, false);
            let scale = Self::property_scale(properties);

            let mut builder = EntityBuilder::new();
            builder.add(PositionComponent::construct(origin));
            builder.add(RotationComponent::construct(rotation));
            builder.add(ScaleComponent::construct(scale));

            builder.add_bundle(Self::entity_line_mesh_instance(entity, properties).build());
            builder.add_bundle(Self::entity_triangle_mesh_instance(entity, properties).build());
            builder.add_bundle(Self::entity_rigid_body(properties).build());
            builder.add_bundle(Self::entity_collider(world, entity, properties, scale).build());
            builder.add_bundle(Self::entity_mover(properties).build());
            builder.add_bundle(Self::entity_event(properties).build());
            builders.push(builder);

            builders.extend(Self::entity_text(properties, origin, scale));
        }

        builders
    }
}

pub fn winit_event_handler<T>(mut f: impl EventLoopHandler<T>) -> impl EventLoopHandler<T> {
    fn prepare_schedule(world: &mut World) {
        assemble_triangle_mesh_instances_system(world);
        assemble_line_mesh_instances_system(world);

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
            antigen_wgpu::buffer_write_slice_system::<TriangleMeshInstanceDataComponent, _>(world);
            antigen_wgpu::buffer_write_slice_system::<LineVertexDataComponent, _>(world);
            antigen_wgpu::buffer_write_slice_system::<LineIndexDataComponent, _>(world);
            antigen_wgpu::buffer_write_slice_system::<LineMeshDataComponent, _>(world);
            antigen_wgpu::buffer_write_slice_system::<LineMeshInstanceDataComponent, _>(world);
            antigen_wgpu::buffer_write_slice_system::<LineInstanceDataComponent, _>(world);
            antigen_wgpu::buffer_write_system::<PositionComponent>(world);
            antigen_wgpu::buffer_write_system::<RotationComponent>(world);
            antigen_wgpu::buffer_write_system::<ScaleComponent>(world);
            antigen_wgpu::buffer_write_system::<LineMeshIdComponent>(world);
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
                phosphor_camera_position_system(world);
            }
            Event::WindowEvent { event, .. } => match event {
                WindowEvent::Resized(_) => {
                    phosphor_resize_system(world);
                }
                //WindowEvent::CursorMoved { .. } => phosphor_cursor_moved_system(world),
                _ => (),
            },
            Event::DeviceEvent { event, .. } => match event {
                DeviceEvent::MouseMotion { delta } => phosphor_mouse_moved_system(world, *delta),
                DeviceEvent::Key(key) => phosphor_key_event_system(world, *key),
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
