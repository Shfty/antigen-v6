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
// TODO: [>] Replace line instances compute shader with storage buffer usage
//           * Bind mesh vertices as storage buffer
//           * Calculate base index as instance_index * 2
//           [ ] Clean up remaining references to compute
//
// TODO: [ ] Line mesh instancing for phosphor renderer
//           * As per line_instancing notes in crate root
//           * Objective is to be able to load each SVG font grapheme once,
//             draw multiple copies without duplicating vertex data
//           * Will require a mesh instance abstraction to encode mesh ID + world position
//           * Should also inform data design for triangle mesh instancing,
//             and provide the basis for loading map entities as individual meshes / ECS entities
//
// TODO: [ ] Improve mesh / line spawning ergonomics
//           * Manually creating a local mutable index and writing it back is too much boilerplate
//           * Solve via wrapper function?
//
// TODO: [ ] Implement generalized render pass setup
//
// TODO: [ ] Implement filesystem thread map loading / building
//           * Need to figure out how to update buffer offsets from entities created off-thread
//           * Send message to render thread, fetch or reserve entity ID
//             * Render thread kicks of load on FS thread, provides ID
//               * FS thread sends components + entity ID to render thread
//                 * Render thread inserts entities with IDs in context

mod demos;

use antigen_core::{receive_messages, try_receive_messages, WorldChannel, WorldExchange};
use antigen_wgpu::wgpu::DeviceDescriptor;
use antigen_winit::EventLoopHandler;
use std::{
    thread::JoinHandle,
    time::{Duration, Instant},
};
use winit::{event::Event, event_loop::ControlFlow, event_loop::EventLoopWindowTarget};

use hecs::World;

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

    game_world.spawn((123, true, "abc"));
    game_world.spawn((42, false));

    // Setup render world
    let mut render_world = World::new();
    render_world.spawn(antigen_winit::BackendBundle::default());
    render_world.spawn(antigen_wgpu::BackendBundle::from_env(
        &DeviceDescriptor {
            label: Some("Device"),
            features: Default::default(),
            limits: Default::default(),
        },
        None,
        None,
    ));

    demos::phosphor::assemble(&mut render_world, &render_channel);

    // Spawn filesystem and game threads
    spawn_world::<Filesystem, _, _>(fs_thread(fs_world, fs_channel));
    spawn_world::<Game, _, _>(game_thread(game_world, game_channel));

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
    move || {
        spin_loop(GAME_THREAD_TICK, || {
            println!("Game");
            try_receive_messages(&mut world, &channel).expect("Error handling message");

            for (id, (number, &flag)) in world.query_mut::<(&mut i32, &bool)>() {
                println!("Entity {:?}", id);
                if flag {
                    *number = number.saturating_mul(2);
                    println!("\tNumber {}", *number);
                }
            }
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
