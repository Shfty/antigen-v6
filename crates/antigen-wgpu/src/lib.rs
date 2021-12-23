mod assemblage;
mod components;
//mod staging_belt;
//mod compute_pass;
mod systems;
mod to_bytes;

use std::path::PathBuf;

use antigen_core::{MessageContext, MessageResult, WorldChannel};
use antigen_fs::FileStringQuery;
use antigen_winit::{
    winit::{
        event::Event,
        event_loop::{ControlFlow, EventLoopWindowTarget},
    },
    EventLoopHandler,
};
pub use assemblage::*;
pub use components::*;
//pub use staging_belt::*;
//pub use compute_pass::*;
use hecs::World;
pub use systems::*;
pub use to_bytes::*;
pub use wgpu;

use wgpu::{BufferAddress, ShaderModuleDescriptor, ShaderSource};

// Return the size of type T in bytes, respresented as a BufferAddress
pub fn buffer_size_of<T>() -> BufferAddress {
    std::mem::size_of::<T>() as BufferAddress
}

// Submit comomand buffers, present surface textures, and drop texture views
pub fn submit_and_present_schedule(world: &mut World) {
    submit_command_buffers_system(world);
    surface_texture_present_system(world);
    surface_texture_view_drop_system(world);
}

fn window_surfaces_schedule(world: &mut World) {
    create_window_surfaces_system(world);
    surface_size_system(world);
    reconfigure_surfaces_system(world);
}

/// Extend an event loop closure with wgpu resource handling
pub fn winit_event_handler<T: Clone>(mut f: impl EventLoopHandler<T>) -> impl EventLoopHandler<T> {
    //let mut staging_belt_manager = StagingBeltManager::new();

    move |world: &mut World,
          channel: &WorldChannel,
          event: Event<'static, T>,
          event_loop_window_target: &EventLoopWindowTarget<T>,
          control_flow: &mut ControlFlow| {
        match event {
            Event::MainEventsCleared => {
                window_surfaces_schedule(world);
                //create_staging_belt_thread_local(&world.read(), &mut staging_belt_manager);
            }
            Event::RedrawRequested(_) => {
                surfaces_textures_views_system(world);
            }
            Event::RedrawEventsCleared => {
                //staging_belt_finish_thread_local(&world.read(), &mut staging_belt_manager);
            }
            _ => (),
        }

        f(
            world,
            channel,
            event.clone(),
            event_loop_window_target,
            control_flow,
        );

        match event {
            Event::MainEventsCleared => {
                //staging_belt_flush_thread_local(&world.read(), &mut staging_belt_manager);
                reset_surface_config_changed_system(world);
            }
            Event::RedrawEventsCleared => {
                submit_and_present_schedule(world);
                //staging_belt_recall_thread_local(&world.read(), &mut staging_belt_manager);
            }
            _ => (),
        }
    }
}

pub fn spawn_shader_from_file_string<'a, 'b, P: Into<PathBuf>>(
    path: P,
) -> impl FnOnce(MessageContext<'a, 'b>) -> MessageResult<'a, 'b> {
    move |mut ctx| {
        let (world, _) = &mut ctx;

        let map_path = path.into();
        println!(
            "Thread {} Looking for file string entities with path {:?}..",
            std::thread::current().name().unwrap(),
            map_path
        );

        let components = world
            .query_mut::<FileStringQuery>()
            .into_iter()
            .filter(|(_, FileStringQuery { path, .. })| ***path == *map_path)
            .map(|(entity, FileStringQuery { string, .. })| {
                println!("Creating shader for entity {:?}", entity);
                (
                    entity,
                    ShaderModuleBundle::new(ShaderModuleDescriptor {
                        label: None,
                        source: ShaderSource::Wgsl(std::borrow::Cow::Owned((**string).clone())),
                    }),
                )
            })
            .collect::<Vec<_>>();

        for (entity, map) in components {
            world
                .insert(entity, map)
                .expect("Failed to add shader to entity");
        }

        Ok(ctx)
    }
}
