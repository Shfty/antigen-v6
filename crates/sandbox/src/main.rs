// TODO: [✓] Refactor antigen-fs and antigen-shambler to use message pattern instead of systems
//
// TODO: [✓] Reimplement component indirection
//
// TODO: [✓] Finish porting phosphor demo
//
// TODO: [✓] Refactor prepare function to avoid unnecessary resource creation
//           * Split into specific sub-functions to ease maintenance
//
// TODO: [✓] Split texture and view components for phosphor demo
//
// TODO: [✓] Refactor wgpu types to remove usage generics
//
// TODO: [✓] Finish generalized compute pipeline dispatch
//
// TODO: [>] Implement generalized render pass dispatch
//           [✓] Draw implementation
//           [✓] Draw indexed implementation
//           [✓] Implement remaining RenderPass parameters
//           [✓] Draw indirect implementation
//           [✓] Draw indexed indirect implementation
//           [ ] Multi-draw implementations
//           [ ] Execute Bundles implementation
//           [ ] Struct parameters for bundle constructors
//               * wgpu descriptors, but with entities instead of references
//           [ ] Builder pattern for RenderPass bundles?
//
// TODO: [✓] Codify buffer flipping as components + systems
//           * Will allow phosphor decay and tonemap to draw via ECS
//           [✓] Phosphor-specific implementation
//           [✓] Generalized implementation for antigen-wgpu
//
// TODO: [✓] Implement command buffer sorting
//           * Order of commands currently depends on ECS iteration order
//           * Best to encode order while recording, more concurrecy-friendly
//           * CommandBufferComponent<T>(BTreeMap<T, CommandBuffer>) where T: PartialOrd ?
//             * Provide T during render pass init
//             * Use type defaults for better ergonomics
//
// TODO: [✓] Update render pass draw ranges via system
//
// TODO: [✓] Replace line instances compute shader with storage buffer usage
//           * Bind mesh vertices as storage buffer
//           * Calculate base index as instance_index * 2
//           [✓] Clean up remaining references to compute
//
// TODO: [✓] Mesh instancing for phosphor renderer
//           * As per line_instancing notes in crate root
//           * Objective is to be able to load each SVG font grapheme once,
//             draw multiple copies without duplicating vertex data
//           * Will require a mesh instance abstraction to encode mesh ID + world position
//           * Should also inform data design for triangle mesh instancing,
//             and provide the basis for loading map entities as individual meshes / ECS entities
//           [✓] First working implementation with new data model
//           [✓] Separate mesh loading and instance creation
//           [✓] Implement text object - read string from map file, spawn grapheme line mesh instances
//           [✓] Instancing for triangle meshes
//               [✓] Fix incorrect instance positioning in beam_mesh vertex shader
//               [✓] Separate instance creation from mesh loading
//           [✓] Load room brush entities as separate meshes
//
// TODO: [✓] Improve mesh / line spawning ergonomics
//           * Manually creating a local mutable index and writing it back is too much boilerplate
//           * Too many state variables to pass around
//           * Is it feasible to read entities from the world when creating a builder?
//             * Ostensibly yes, since the count components are fetched by calling code
//
// TODO: [✓] Rotation and scale support for triangle and line meshes
//           * [✓] Use quaternions for rotation
//           * [✓] Vec3 for scale
//
// TODO: [✓] Respect angle and mangle when spawning point entities
//           * Will need to convert from quake-forward to wgpu-forward
//
// TODO: [✓] Stratify mesh loading
//           * Need to be able to create mesh instances by name instead of manually caching IDs
//           [✓] Store name-id map as component, write during mesh load, lookup during instancing
//
// TODO: [✓] Separate triangle / line mesh instance position, rotation, scale out into distinct components
//           * Should be able to create a single BufferWrite per member with appropriate offsets
//
// TODO: [✓] Implement filesystem thread map loading / building
//           * Need to be able to read and write buffers from different threads
//           * Use Arc<Buffer> and clone between threads
//             * Render thread holds buffers, meshes, render passes
//             * Game thread holds buffers, mesh instances
//             * Create a RemoteComponent<T> abstraction for sharing components across threads
//           [✓] Separate oscilloscope mesh creation from instancing
//           [✓] Separate test geo triangle mesh creation from instancing
//           [✓] Use Arc + RwLock around buffer LazyComponent to avoid having to force-create buffers before send
//           [✓] Reduce boilerplate for cross-thread setup
//               * Too much repetition in phosphor mod.rs
//           [✓] Move map processing to filesystem thread
//
// TODO: [ ] Integrate rapier physics
//           * Create collision from brush hulls
//
// TODO: [ ] Figure out why lower-case z is missing from text test
//
// TODO: [ ] Separate box bot from player start
//           * Player start should represent the camera for now
//           * Implement as a box_bot point entity
//
// TODO: [ ] Implement camera abstraction
//           [ ] Spawn at first player start
//           [ ] Mouse capture
//           [ ] First-person controls
//
// TODO: [ ] Implement compute-based frustum culling
//
// TODO: [ ] Implement generalized render pass setup
//
// TODO: [ ] Implement portal rendering
//           * Ideally all portal rendering should happen in existing draw calls for performance's sake
//               * Just add more geometry
//                 * Effectively an extra layer of room -> mesh instances indirection
//                 * Re-instance rooms and their contents when viewed through a portal
//               * Stencil buffer seems the best approach to early-out from invisible fragments
//               * Each portal recursion adds 1 to the stencil value
//               * Use less-than stencil comparator
//          * Will need a way to track the current room in order to begin portal traversal
//            * Point-in-box checks against room hulls
//            * If camera is not inside a room, find the closest one
//              * Ideally should use distance-to-nearest-surface
//              * If impractical, distance-to-center should suffice
//
// TODO: [ ] Investigate box portals for room-inside-room
//
// TODO: [ ] Generalize map -> entities + components conversion
//           * Need a way to map classname to a set of entities, properties to components
//           * Multiple cases to consider:
//             * Simple single-value components
//               * Parse single property via FromStr
//             * Complex multi-value components
//               * Parse multiple properties via FromStr
//               * Use component.member naming for TrenchBroom properties
//           * Traits + cons lists to model classname -> components relation?
//
// TODO: [ ] Implement bloom pass
//
//

mod demos;

use antigen_core::{
    receive_messages, send_clone_query, try_receive_messages, PositionComponent,
    TaggedEntitiesComponent, WorldChannel, WorldExchange, Construct, RotationComponent
};
use antigen_wgpu::{
    wgpu::DeviceDescriptor, AdapterComponent, DeviceComponent, InstanceComponent, QueueComponent,
};
use antigen_winit::EventLoopHandler;
use rapier3d::prelude::{ColliderBuilder, RigidBodyBuilder};
use std::{
    thread::JoinHandle,
    time::{Duration, Instant},
};
use winit::{event::Event, event_loop::ControlFlow, event_loop::EventLoopWindowTarget};

use hecs::World;

use antigen_rapier3d::{physics_backend_builder, ColliderComponent, RigidBodyComponent};

const GAME_THREAD_TICK: Duration = Duration::from_nanos(16670000);

enum Game {}
enum Render {}
enum Filesystem {}

fn main() {
    //tracing_subscriber::fmt::fmt().pretty().init();

    // Create world exchange
    let mut exchange = WorldExchange::default();

    // Create thread-specific channels
    let fs_channel = exchange.create_channel::<Filesystem>();
    let game_channel = exchange.create_channel::<Game>();
    let render_channel = exchange.create_channel::<Render>();

    // Spawn exchange into its own thread
    exchange.spawn();

    // Setup filesystem world
    let fs_world = World::new();

    // Setup game world
    let mut game_world = World::new();
    game_world.spawn((TaggedEntitiesComponent::default(),));
    game_world.spawn((123, true, "abc"));
    game_world.spawn((42, false));

    // Spawn filesystem and game threads
    spawn_world::<Filesystem, _, _>(fs_thread(fs_world, fs_channel));
    spawn_world::<Game, _, _>(game_thread(game_world, game_channel));

    // Setup render world
    let mut render_world = World::new();
    render_world.spawn((TaggedEntitiesComponent::default(),));
    render_world.spawn(antigen_winit::BackendBundle::default());
    let wgpu_backend_entity = render_world.spawn(antigen_wgpu::BackendBundle::from_env(
        &DeviceDescriptor {
            label: Some("Device"),
            features: Default::default(),
            limits: Default::default(),
        },
        None,
        None,
    ));

    // Clone WGPU backend components to game thread
    send_clone_query::<
        (
            &InstanceComponent,
            &AdapterComponent,
            &DeviceComponent,
            &QueueComponent,
        ),
        Game,
    >(wgpu_backend_entity)((&mut render_world, &render_channel))
    .unwrap();

    // Assemble phosphor renderer
    demos::phosphor::assemble(&mut render_world, &render_channel);

    // Enter winit event loop
    winit::event_loop::EventLoop::new().run(antigen_winit::wrap_event_loop(
        render_world,
        render_channel,
        antigen_winit::winit_event_handler(antigen_wgpu::winit_event_handler(
            demos::phosphor::winit_event_handler(render_thread(
                antigen_winit::winit_event_terminator(),
            )),
        )),
    ));
}

/// Spawn a thread with a world and function entrypoint
fn spawn_world<U, F, R>(f: F) -> JoinHandle<R>
where
    F: FnOnce() -> R + Send + 'static,
    R: Send + 'static,
{
    std::thread::Builder::new()
        .name(std::any::type_name::<U>().into())
        .spawn(f)
        .unwrap()
}

/// Runs `f` at  `duration` intervals using a spin-lock for timing
fn spin_loop<F: FnMut()>(duration: Duration, mut f: F) -> ! {
    let mut ts = Instant::now();
    loop {
        f();
        while Instant::now().duration_since(ts) < duration {
            std::hint::spin_loop();
        }
        ts = Instant::now();
    }
}

/// Filesystem thread
fn fs_thread(mut world: World, channel: WorldChannel) -> impl FnMut() {
    move || loop {
        receive_messages(&mut world, &channel).expect("Error receiving message");
    }
}

/// Game thread
fn game_thread(mut world: World, channel: WorldChannel) -> impl FnMut() {
    // Create the physics backend
    world.spawn(physics_backend_builder().build());

    // Create the ground
    world.spawn((ColliderComponent::new(
        ColliderBuilder::cuboid(100.0, 0.1, 100.0).build(),
    ),));

    // Create the bounding ball.
    let rigid_body_entity = world.spawn((
        RigidBodyComponent::new(RigidBodyBuilder::new_dynamic().build()),
        PositionComponent::construct(nalgebra::vector![0.0, 10.0, 0.0]),
        RotationComponent::construct(nalgebra::UnitQuaternion::identity()),
    ));

    world.spawn((ColliderComponent::new_with_parent(
        ColliderBuilder::ball(0.5).restitution(0.7).build(),
        rigid_body_entity,
    ),));

    move || {
        spin_loop(GAME_THREAD_TICK, || {
            try_receive_messages(&mut world, &channel).expect("Error handling message");

            antigen_rapier3d::insert_colliders_system(&mut world);
            antigen_rapier3d::insert_rigid_bodies_system(&mut world);
            antigen_rapier3d::step_physics_system(&mut world);
            antigen_rapier3d::read_back_rigid_body_isometries_system(&mut world);

            antigen_wgpu::buffer_write_slice_system::<
                demos::phosphor::TriangleMeshInstanceDataComponent,
                _,
            >(&mut world);
            antigen_wgpu::buffer_write_slice_system::<
                demos::phosphor::LineMeshInstanceDataComponent,
                _,
            >(&mut world);
            antigen_wgpu::buffer_write_slice_system::<demos::phosphor::LineInstanceDataComponent, _>(
                &mut world,
            );
            antigen_wgpu::buffer_write_system::<antigen_core::PositionComponent>(&mut world);
            antigen_wgpu::buffer_write_system::<antigen_core::RotationComponent>(&mut world);
            antigen_wgpu::buffer_write_system::<antigen_core::ScaleComponent>(&mut world);
            antigen_wgpu::buffer_write_system::<demos::phosphor::LineMeshIdComponent>(&mut world);
        })
    }
}

/// Render thread
pub fn render_thread<T: Clone>(mut f: impl EventLoopHandler<T>) -> impl EventLoopHandler<T> {
    move |world: &mut World,
          channel: &WorldChannel,
          event: Event<'static, T>,
          event_loop_window_target: &EventLoopWindowTarget<T>,
          control_flow: &mut ControlFlow| {
        try_receive_messages(world, channel).expect("Error handling message");

        match event {
            winit::event::Event::MainEventsCleared => {
                println!("Main events cleared");
            }
            _ => (),
        }

        f(
            world,
            channel,
            event,
            event_loop_window_target,
            control_flow,
        )
    }
}
