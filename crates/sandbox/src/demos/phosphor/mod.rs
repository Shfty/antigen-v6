// TODO: [✓] Evaluate gradient before phosphor front buffer is written
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
//           [ ] Paralellize shambler
//
//       [ ] Figure out how to flush command buffers at runtime
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
mod systems;

use antigen_fs::{load_file_string, FilePathComponent};
pub use assemblage::*;
pub use components::*;
pub use render_passes::*;
pub use systems::*;

use expression::EvalTrait;
use std::{collections::BTreeMap, path::PathBuf, time::Instant};

use antigen_winit::{
    winit::{
        event::{Event, WindowEvent},
        event_loop::{ControlFlow, EventLoopWindowTarget},
    },
    EventLoopHandler, RedrawUnconditionally, WindowComponent,
};

use antigen_core::{
    send_component, Construct, Indirect, Lift, MessageContext, MessageResult, SendTo, WorldChannel,
};

use antigen_wgpu::{
    buffer_size_of, spawn_shader_from_file_string,
    wgpu::{
        AddressMode, BufferAddress, BufferDescriptor, BufferUsages, CommandEncoderDescriptor,
        ComputePassDescriptor, Extent3d, FilterMode, Maintain, SamplerDescriptor, TextureAspect,
        TextureDescriptor, TextureDimension, TextureFormat, TextureUsages, TextureViewDescriptor,
    },
    BindGroupComponent, BindGroupLayoutComponent, ComputePipelineComponent,
    RenderAttachmentTextureView, RenderPipelineComponent, ShaderModuleComponent,
    ShaderModuleDescriptorComponent, SurfaceConfigurationComponent,
};

use antigen_shambler::shambler::GeoMap;

use hecs::{Entity, EntityBuilder, World};

use crate::{Filesystem, Render};

const HDR_TEXTURE_FORMAT: TextureFormat = TextureFormat::Rgba16Float;
const MAX_MESH_VERTICES: usize = 10000;
const MAX_MESH_INDICES: usize = 10000;
const MAX_LINE_INDICES: usize = 20000;
const MAX_LINES: usize = MAX_LINE_INDICES / 2;
const CLEAR_COLOR: antigen_wgpu::wgpu::Color = antigen_wgpu::wgpu::Color {
    r: 0.0,
    g: 0.0,
    b: 0.0,
    a: -8.0,
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

pub fn assemble(world: &mut World, channel: &WorldChannel) {
    let window_entity = world.reserve_entity();
    let renderer_entity = world.reserve_entity();

    // Uniforms
    let mut builder = EntityBuilder::new();
    let bundle = builder
        .add(Uniform)
        .add(BindGroupLayoutComponent::default())
        .add(BindGroupComponent::default())
        .add_bundle(antigen_wgpu::BufferBundle::new(BufferDescriptor {
            label: Some("Uniform Buffer"),
            size: buffer_size_of::<UniformData>(),
            usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
            mapped_at_creation: false,
        }))
        .build();
    let uniform_entity = world.spawn(bundle);

    // Mesh Vertices
    let mut builder = EntityBuilder::new();
    let bundle = builder
        .add(MeshVertex)
        .add_bundle(antigen_wgpu::BufferBundle::new(BufferDescriptor {
            label: Some("Mesh Vertex Buffer"),
            size: buffer_size_of::<MeshVertexData>() * MAX_MESH_VERTICES as BufferAddress,
            usage: BufferUsages::VERTEX | BufferUsages::STORAGE | BufferUsages::COPY_DST,
            mapped_at_creation: false,
        }))
        .build();
    let mesh_vertex_entity = world.spawn(bundle);

    // Mesh Indices
    let mut builder = EntityBuilder::new();
    let bundle = builder
        .add(MeshIndex)
        .add_bundle(antigen_wgpu::BufferBundle::new(BufferDescriptor {
            label: Some("Mesh Index Buffer"),
            size: buffer_size_of::<u16>() * MAX_MESH_INDICES as BufferAddress,
            usage: BufferUsages::INDEX | BufferUsages::COPY_DST,
            mapped_at_creation: false,
        }))
        .build();
    let mesh_index_entity = world.spawn(bundle);

    // Line Vertices
    let vertices = circle_strip(2);
    let mut builder = EntityBuilder::new();
    let line_vertex_entity = world.reserve_entity();
    let bundle = builder
        .add(LineVertex)
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
        .add(LineIndex)
        .add_bundle(antigen_wgpu::BufferBundle::new(BufferDescriptor {
            label: Some("Line Index Buffer"),
            size: buffer_size_of::<u32>() * MAX_LINE_INDICES as BufferAddress,
            usage: BufferUsages::STORAGE | BufferUsages::COPY_DST,
            mapped_at_creation: false,
        }))
        .build();
    let line_index_entity = world.spawn(bundle);

    // Line Instances
    let mut builder = EntityBuilder::new();
    let bundle = builder
        .add(LineInstance)
        .add_bundle(antigen_wgpu::BufferBundle::new(BufferDescriptor {
            label: Some("Line Instance Buffer"),
            size: buffer_size_of::<LineInstanceData>() * MAX_LINES as BufferAddress,
            usage: BufferUsages::VERTEX | BufferUsages::STORAGE | BufferUsages::COPY_DST,
            mapped_at_creation: false,
        }))
        .build();
    let _line_instance_entity = world.spawn(bundle);

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
    let _beam_buffer_entity = world.spawn(bundle);

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
    let _beam_depth_buffer_entity = world.spawn(bundle);

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
    let _beam_multisample_entity = world.spawn(bundle);

    // Phosphor front buffer
    let mut builder = EntityBuilder::new();
    let bundle = builder
        .add(PhosphorFrontBuffer)
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
        .build();
    let _beam_multisample_entity = world.spawn(bundle);

    // Phosphor back buffer
    let mut builder = EntityBuilder::new();
    let bundle = builder
        .add(PhosphorBackBuffer)
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
    let _beam_multisample_entity = world.spawn(bundle);

    // Assemble window
    let mut builder = EntityBuilder::new();
    let bundle = builder
        .add_bundle(antigen_winit::WindowBundle::default())
        .add_bundle(antigen_winit::WindowTitleBundle::new("Phosphor"))
        .add_bundle(antigen_wgpu::WindowSurfaceBundle::new())
        .add(RedrawUnconditionally)
        .build();
    world.insert(window_entity, bundle).unwrap();

    // Compute pass
    let compute_pass_entity = world.spawn((
        ComputeLineInstances,
        ComputePipelineComponent::default(),
        BindGroupLayoutComponent::default(),
        BindGroupComponent::default(),
    ));

    load_shader::<Filesystem, _>(
        channel,
        compute_pass_entity,
        "crates/sandbox/src/demos/phosphor/shaders/line_instances.wgsl",
    );

    // Phosphor pass
    let phosphor_pass_entity = world.spawn((
        PhosphorDecay,
        RenderPipelineComponent::default(),
        BindGroupLayoutComponent::default(),
    ));

    // Shaders
    load_shader::<Filesystem, _>(
        channel,
        phosphor_pass_entity,
        "crates/sandbox/src/demos/phosphor/shaders/phosphor_decay.wgsl",
    );

    // Front buffer
    let _phosphor_front_entity = world.spawn((PhosphorFrontBuffer, BindGroupComponent::default()));

    // Back buffer
    let _phosphor_back_entity = world.spawn((PhosphorBackBuffer, BindGroupComponent::default()));

    // Beam mesh pass
    let beam_mesh_pass_entity = world.spawn((BeamMesh, RenderPipelineComponent::default()));

    load_shader::<Filesystem, _>(
        channel,
        beam_mesh_pass_entity,
        "crates/sandbox/src/demos/phosphor/shaders/beam_mesh.wgsl",
    );

    // Beam line pass
    let beam_line_pass_entity = world.spawn((BeamLine, RenderPipelineComponent::default()));

    load_shader::<Filesystem, _>(
        channel,
        beam_line_pass_entity,
        "crates/sandbox/src/demos/phosphor/shaders/beam_line.wgsl",
    );

    // Tonemap pass
    let tonemap_pass_entity = world.spawn((Tonemap, RenderPipelineComponent::default()));

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

    builder.add_bundle(antigen_wgpu::CommandEncoderBundle::new(
        CommandEncoderDescriptor {
            label: Some("Phosphor Encoder"),
        },
        renderer_entity,
    ));

    builder.add_bundle(
        antigen_wgpu::ComputePassBundle::builder(
            ComputePassDescriptor {
                label: Some("Line Indices".into()),
            },
            compute_pass_entity,
            vec![(compute_pass_entity, vec![])],
            vec![],
            (177, 1, 1),
        )
        .build(),
    );

    // Misc
    builder
        .add(antigen_wgpu::CommandBuffersComponent::default())
        .add(BufferFlipFlopComponent::construct(false))
        // Indirect surface config and view for resize handling
        .add(Indirect::<&SurfaceConfigurationComponent>::construct(
            window_entity,
        ))
        .add(Indirect::<&RenderAttachmentTextureView>::construct(
            window_entity,
        ))
        // Indirect window for input handling
        .add(Indirect::<&WindowComponent>::construct(window_entity));

    // Assemble geometry
    let mut vertex_head = 0;
    let mut line_index_head = 0;
    let mut mesh_index_head = 0;

    println!("Mesh vertex entity: {:?}", mesh_vertex_entity);
    println!("Line index entity: {:?}", line_index_entity);

    assemble_test_geometry(
        world,
        mesh_vertex_entity,
        line_index_entity,
        &mut vertex_head,
        &mut line_index_head,
    );

    /*
    load_map::<MapFile, Filesystem, _>(
        channel,
        renderer_entity,
        "crates/sandbox/src/demos/phosphor/maps/index_align_test.map",
    );
    */

    let map_file = include_str!("maps/line_index_test.map");
    let map = map_file
        .parse::<antigen_shambler::shambler::shalrath::repr::Map>()
        .unwrap();
    let geo_map = GeoMap::from(map);
    let map_data = MapData::from(geo_map);

    let mut visual_brushes = map_data.build_visual_brushes(
        mesh_vertex_entity,
        mesh_index_entity,
        line_index_entity,
        &mut vertex_head,
        &mut mesh_index_head,
        &mut line_index_head,
    );
    let bundles = visual_brushes.iter_mut().map(EntityBuilder::build);
    world.extend(bundles);

    let mut point_entities = map_data.build_point_entities(
        mesh_vertex_entity,
        mesh_index_entity,
        line_index_entity,
        &mut vertex_head,
        &mut mesh_index_head,
        &mut line_index_head,
    );
    let bundles = point_entities.iter_mut().map(EntityBuilder::build);
    world.extend(bundles);

    // Store mesh and line index counts for render system
    let vertex_count = VertexCountComponent::construct(vertex_head);
    let mesh_index_count = MeshIndexCountComponent::construct(mesh_index_head);
    let line_index_count = LineIndexCountComponent::construct(line_index_head);

    builder.add_bundle((vertex_count, mesh_index_count, line_index_count));

    // Done
    let bundle = builder.build();
    world.insert(renderer_entity, bundle).unwrap();
}

fn assemble_test_geometry(
    world: &mut World,
    mesh_vertex_entity: Entity,
    line_index_entity: Entity,
    vertex_head: &mut BufferAddress,
    line_index_head: &mut BufferAddress,
) {
    // Oscilloscopes
    world.spawn(
        OscilloscopeBundle::builder(
            mesh_vertex_entity,
            line_index_entity,
            vertex_head,
            line_index_head,
            (-80.0, 40.0, -80.0),
            RED,
            Oscilloscope::new(3.33, 30.0, |f| (f.sin(), f.cos(), f.sin())),
            2.0,
            -1.0,
        )
        .build(),
    );

    world.spawn(
        OscilloscopeBundle::builder(
            mesh_vertex_entity,
            line_index_entity,
            vertex_head,
            line_index_head,
            (-80.0, 40.0, 0.0),
            GREEN,
            Oscilloscope::new(2.22, 30.0, |f| (f.sin(), (f * 1.2).sin(), (f * 1.4).cos())),
            2.0,
            -2.0,
        )
        .build(),
    );

    world.spawn(
        OscilloscopeBundle::builder(
            mesh_vertex_entity,
            line_index_entity,
            vertex_head,
            line_index_head,
            (-80.0, 40.0, 80.0),
            BLUE,
            Oscilloscope::new(3.33, 30.0, |f| (f.cos(), (f * 1.2).cos(), (f * 1.4).cos())),
            2.0,
            -4.0,
        )
        .build(),
    );

    // Gradient 3 Triangle
    world.spawn(
        LineStripBundle::builder(
            mesh_vertex_entity,
            line_index_entity,
            vertex_head,
            line_index_head,
            vec![
                MeshVertexData::new((-50.0, -20.0, 0.0), RED, RED, 5.0, -20.0),
                MeshVertexData::new((-90.0, -80.0, 0.0), GREEN, GREEN, 4.0, -20.0),
                MeshVertexData::new((-10.0, -80.0, 0.0), BLUE, BLUE, 3.0, -20.0),
                MeshVertexData::new((-50.0, -20.0, 0.0), RED, RED, 2.0, -20.0),
            ],
        )
        .build(),
    );

    // Gradients 0-2 Triangle
    world.spawn(
        LineStripBundle::builder(
            mesh_vertex_entity,
            line_index_entity,
            vertex_head,
            line_index_head,
            vec![
                MeshVertexData::new((50.0, -80.0, 0.0), BLUE, BLUE, 7.0, -10.0),
                MeshVertexData::new((90.0, -20.0, 0.0), BLUE, BLUE, 6.0, -10.0),
                MeshVertexData::new((90.0, -20.0, 0.0), GREEN, GREEN, 5.0, -10.0),
                MeshVertexData::new((10.0, -20.0, 0.0), GREEN, GREEN, 4.0, -10.0),
                MeshVertexData::new((10.0, -20.0, 0.0), RED, RED, 3.0, -10.0),
                MeshVertexData::new((50.0, -80.0, 0.0), RED, RED, 2.0, -10.0),
            ],
        )
        .build(),
    );
}

struct MapData {
    geo_map: antigen_shambler::shambler::GeoMap,
    face_planes: antigen_shambler::shambler::face::FacePlanes,
    brush_hulls: antigen_shambler::shambler::brush::BrushHulls,
    face_vertices: antigen_shambler::shambler::face::FaceVertices,
    face_duplicates: antigen_shambler::shambler::face::FaceDuplicates,
    face_centers: antigen_shambler::shambler::face::FaceCenters,
    face_indices: antigen_shambler::shambler::face::FaceIndices,
    face_triangle_indices: antigen_shambler::shambler::face::FaceTriangleIndices,
    face_line_indices: antigen_shambler::shambler::line::Lines,
    interior_faces: antigen_shambler::shambler::face::InteriorFaces,
    face_bases: antigen_shambler::shambler::face::FaceBases,
    face_face_containment: antigen_shambler::shambler::face::FaceFaceContainment,
    brush_face_containment: antigen_shambler::shambler::brush::BrushFaceContainment,
}

impl From<GeoMap> for MapData {
    fn from(geo_map: GeoMap) -> Self {
        // Create geo planes from brush planes
        let face_planes = antigen_shambler::shambler::face::FacePlanes::new(&geo_map.face_planes);

        // Create per-brush hulls from brush planes
        let brush_hulls =
            antigen_shambler::shambler::brush::BrushHulls::new(&geo_map.brush_faces, &face_planes);

        // Generate face vertices
        let face_vertices = antigen_shambler::shambler::face::FaceVertices::new(
            &geo_map.brush_faces,
            &face_planes,
            &brush_hulls,
        );

        // Find duplicate faces
        let face_duplicates = antigen_shambler::shambler::face::FaceDuplicates::new(
            &geo_map.faces,
            &face_planes,
            &face_vertices,
        );

        // Generate centers
        let face_centers = antigen_shambler::shambler::face::FaceCenters::new(&face_vertices);

        // Generate per-plane CCW face indices
        let face_indices = antigen_shambler::shambler::face::FaceIndices::new(
            &geo_map.face_planes,
            &face_planes,
            &face_vertices,
            &face_centers,
            antigen_shambler::shambler::face::FaceWinding::Clockwise,
        );

        let face_triangle_indices =
            antigen_shambler::shambler::face::FaceTriangleIndices::new(&face_indices);
        let face_line_indices = antigen_shambler::shambler::line::Lines::new(&face_indices);

        let interior_faces = antigen_shambler::shambler::face::InteriorFaces::new(
            &geo_map.entity_brushes,
            &geo_map.brush_faces,
            &face_duplicates,
            &face_vertices,
            &face_line_indices,
        );

        // Generate tangents
        let face_bases = antigen_shambler::shambler::face::FaceBases::new(
            &geo_map.faces,
            &face_planes,
            &geo_map.face_offsets,
            &geo_map.face_angles,
            &geo_map.face_scales,
        );

        // Calculate face-face containment
        let face_face_containment = antigen_shambler::shambler::face::FaceFaceContainment::new(
            &geo_map.faces,
            &face_planes,
            &face_bases,
            &face_vertices,
            &face_line_indices,
        );

        // Calculate brush-face containment
        let brush_face_containment = antigen_shambler::shambler::brush::BrushFaceContainment::new(
            &geo_map.brushes,
            &geo_map.faces,
            &geo_map.brush_faces,
            &brush_hulls,
            &face_vertices,
        );

        MapData {
            geo_map,
            face_planes,
            brush_hulls,
            face_vertices,
            face_duplicates,
            face_centers,
            face_indices,
            face_triangle_indices,
            face_line_indices,
            interior_faces,
            face_bases,
            face_face_containment,
            brush_face_containment,
        }
    }
}

impl MapData {
    pub fn build_visual_brushes(
        &self,
        mesh_vertex_entity: Entity,
        mesh_index_entity: Entity,
        line_index_entity: Entity,
        vertex_head: &mut BufferAddress,
        mesh_index_head: &mut BufferAddress,
        line_index_head: &mut BufferAddress,
    ) -> Vec<EntityBuilder> {
        // Generate mesh
        let mut mesh_vertices: Vec<MeshVertexData> = Default::default();
        let mut mesh_indices: Vec<u16> = Default::default();
        let mut line_indices: Vec<u32> = Default::default();

        let scale_factor = 1.0;

        // Gather mesh and line geometry
        let mut face_index_head = *vertex_head as u16;
        for face_id in &self.geo_map.faces {
            if self.face_duplicates.contains(&face_id) {
                continue;
            }

            if self.face_face_containment.is_contained(&face_id) {
                continue;
            }

            if self.brush_face_containment.is_contained(&face_id) {
                continue;
            }

            if !self.interior_faces.contains(&face_id) {
                continue;
            }

            // Fetch and interpret texture data
            let texture_id = self.geo_map.face_textures[&face_id];
            let texture_name = &self.geo_map.textures[&texture_id];

            let color = if texture_name.contains("blood") {
                RED
            } else if texture_name.contains("green") {
                GREEN
            } else if texture_name.contains("blue") {
                BLUE
            } else {
                WHITE
            };

            let intensity = if texture_name.ends_with("3") {
                0.25
            } else if texture_name.ends_with("2") {
                0.375
            } else if texture_name.ends_with("1") {
                0.5
            } else {
                0.125
            };

            let face_vertices = self.face_vertices.vertices(&face_id).unwrap();
            let vertices = face_vertices
                .iter()
                .map(|v| MeshVertexData {
                    position: [v.x * scale_factor, v.z * scale_factor, v.y * scale_factor],
                    surface_color: [0.0, 0.0, 0.0],
                    line_color: [color.0, color.1, color.2],
                    intensity,
                    delta_intensity: -8.0,
                    ..Default::default()
                })
                .collect::<Vec<_>>();
            mesh_vertices.extend(vertices);

            let face_triangle_indices = self.face_triangle_indices.get(&face_id).unwrap();
            let triangle_indices = face_triangle_indices
                .iter()
                .map(|i| *i as u16 + face_index_head)
                .collect::<Vec<_>>();
            mesh_indices.extend(triangle_indices);

            let face_lines = &self.face_line_indices.face_lines[&face_id];
            let face_line_indices = face_lines
                .iter()
                .flat_map(|line_id| {
                    let antigen_shambler::shambler::line::LineIndices { v0, v1 } =
                        self.face_line_indices.line_indices[line_id];
                    [
                        (v0 + face_index_head as usize) as u32,
                        (v1 + face_index_head as usize) as u32,
                    ]
                })
                .collect::<Vec<_>>();
            line_indices.extend(face_line_indices);

            face_index_head += face_vertices.len() as u16;
        }

        vec![
            MeshBundle::builder(
                mesh_vertex_entity,
                mesh_index_entity,
                vertex_head,
                mesh_index_head,
                mesh_vertices,
                mesh_indices,
            ),
            LineIndicesBundle::builder(line_index_entity, line_index_head, line_indices),
        ]
    }

    pub fn build_point_entities(
        &self,
        mesh_vertex_entity: Entity,
        mesh_index_entity: Entity,
        line_index_entity: Entity,
        vertex_head: &mut BufferAddress,
        mesh_index_head: &mut BufferAddress,
        line_index_head: &mut BufferAddress,
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
            let origin = player_start.0.iter().find(|p| p.key == "origin").unwrap();
            let mut origin = origin.value.split_whitespace();
            let x = origin.next().unwrap().parse::<f32>().unwrap();
            let y = origin.next().unwrap().parse::<f32>().unwrap();
            let z = origin.next().unwrap().parse::<f32>().unwrap();
            builders.extend(
                BoxBotBundle::builders(
                    mesh_vertex_entity,
                    mesh_index_entity,
                    line_index_entity,
                    vertex_head,
                    mesh_index_head,
                    line_index_head,
                    (x, z, y),
                )
                .into_iter(),
            );
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
            let origin = oscilloscope.0.iter().find(|p| p.key == "origin").unwrap();
            let mut origin = origin.value.split_whitespace();
            let x = origin.next().unwrap().parse::<f32>().unwrap();
            let z = origin.next().unwrap().parse::<f32>().unwrap();
            let y = origin.next().unwrap().parse::<f32>().unwrap();
            let origin = (x, y, z);

            let color = oscilloscope.0.iter().find(|p| p.key == "color").unwrap();
            let mut color = color.value.split_whitespace();
            let x = color.next().unwrap().parse::<f32>().unwrap();
            let z = color.next().unwrap().parse::<f32>().unwrap();
            let y = color.next().unwrap().parse::<f32>().unwrap();
            let color = (x, y, z);

            let intensity = oscilloscope
                .0
                .iter()
                .find(|p| p.key == "intensity")
                .unwrap()
                .value
                .parse::<f32>()
                .unwrap();
            let delta_intensity = oscilloscope
                .0
                .iter()
                .find(|p| p.key == "delta_intensity")
                .unwrap()
                .value
                .parse::<f32>()
                .unwrap();
            let speed = oscilloscope
                .0
                .iter()
                .find(|p| p.key == "speed")
                .unwrap()
                .value
                .parse::<f32>()
                .unwrap();
            let magnitude = oscilloscope
                .0
                .iter()
                .find(|p| p.key == "magnitude")
                .unwrap()
                .value
                .parse::<f32>()
                .unwrap();

            let x = &oscilloscope.0.iter().find(|p| p.key == "x").unwrap().value;
            let x = expression::parse_expression(x);

            let y = &oscilloscope.0.iter().find(|p| p.key == "y").unwrap().value;
            let y = expression::parse_expression(y);

            let z = &oscilloscope.0.iter().find(|p| p.key == "z").unwrap().value;
            let z = expression::parse_expression(z);

            builders.push(OscilloscopeBundle::builder(
                mesh_vertex_entity,
                line_index_entity,
                vertex_head,
                line_index_head,
                origin,
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
            antigen_wgpu::buffer_write_system::<Vec<LineVertexData>>(world);
            antigen_wgpu::buffer_write_system::<Vec<u32>>(world);
            antigen_wgpu::buffer_write_system::<Vec<MeshVertexData>>(world);
            antigen_wgpu::buffer_write_system::<MeshIndexDataComponent>(world);
        }
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
        antigen_wgpu::dispatch_compute_passes_system(world);
        phosphor_render_system(world);
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
