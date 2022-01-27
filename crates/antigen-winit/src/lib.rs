mod assemblage;
mod components;
mod systems;

pub use assemblage::*;
pub use components::*;
pub use systems::*;

pub use winit;

use winit::{
    event::{DeviceEvent, DeviceId, Event, WindowEvent},
    event_loop::{ControlFlow, EventLoopWindowTarget},
    window::WindowId,
};

use hecs::World;

use antigen_core::WorldChannel;

/// A winit-compatible event loop closure
pub trait WinitEventLoopHandler<T>:
    FnMut(Event<T>, &EventLoopWindowTarget<T>, &mut ControlFlow)
{
}
impl<T, U> WinitEventLoopHandler<U> for T where
    T: FnMut(Event<U>, &EventLoopWindowTarget<U>, &mut ControlFlow)
{
}

/// A composable antigen_winit event loop closure
pub trait EventLoopHandler<T>:
    FnMut(&mut World, &WorldChannel, Event<'static, T>, &EventLoopWindowTarget<T>, &mut ControlFlow)
{
}

impl<T, U> EventLoopHandler<U> for T where
    T: FnMut(
        &mut World,
        &WorldChannel,
        Event<'static, U>,
        &EventLoopWindowTarget<U>,
        &mut ControlFlow,
    )
{
}

/// Wrap [`EventLoopHandler`] into a [`WinitEventLoopHandler`]
pub fn wrap_event_loop<T>(
    mut world: World,
    channel: WorldChannel,
    mut f: impl EventLoopHandler<T>,
) -> impl WinitEventLoopHandler<T> {
    move |event: Event<T>,
          event_loop_window_target: &EventLoopWindowTarget<T>,
          control_flow: &mut winit::event_loop::ControlFlow| {
        let event = if let Some(event) = event.to_static() {
            event
        } else {
            return;
        };

        f(
            &mut world,
            &channel,
            event,
            event_loop_window_target,
            control_flow,
        )
    }
}

fn get_window_event_component(
    world: &mut World,
) -> &mut (Option<WindowId>, Option<WindowEvent<'static>>) {
    let (_, window_event) = world
        .query_mut::<&mut WindowEventComponent>()
        .into_iter()
        .next()
        .unwrap();
    window_event
}

fn get_device_event_component(world: &mut World) -> &mut (Option<DeviceId>, Option<DeviceEvent>) {
    let (_, device_event) = world
        .query_mut::<&mut DeviceEventComponent>()
        .into_iter()
        .next()
        .unwrap();
    device_event
}

/// Extend an event loop closure with ECS event loop handling and window functionality
pub fn winit_event_handler<T: Clone>(mut f: impl EventLoopHandler<T>) -> impl EventLoopHandler<T> {
    move |world: &mut World,
          channel: &WorldChannel,
          event: Event<'static, T>,
          event_loop_window_target: &EventLoopWindowTarget<T>,
          control_flow: &mut ControlFlow| {
        {
            let window_event = get_window_event_component(world);
            *window_event = (None, None);
        }

        {
            let device_event = get_device_event_component(world);
            *device_event = (None, None);
        }

        match &event {
            winit::event::Event::MainEventsCleared => {
                create_windows_system(world, event_loop_window_target);
                window_title_system(world);
                redraw_unconditionally_system(world);
            }
            winit::event::Event::RedrawRequested(window_id) => {
                get_window_event_component(world).0 = Some(*window_id);
            }
            winit::event::Event::WindowEvent { window_id, event } => {
                *get_window_event_component(world) = (Some(*window_id), Some(event.clone()));
                match event {
                    WindowEvent::Resized(_) => {
                        resize_window_system(world);
                    }
                    WindowEvent::CloseRequested => {
                        close_window_system(world);
                    }
                    _ => (),
                }
            }
            winit::event::Event::DeviceEvent { device_id, event } => {
                *get_device_event_component(world) = (Some(*device_id), Some(event.clone()))
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

        match &event {
            winit::event::Event::MainEventsCleared => {
                reset_window_size_changed_system(world);
            }
            _ => (),
        }
    }
}

/// Unit winit event handler
pub fn winit_event_terminator<T>() -> impl EventLoopHandler<T> {
    move |_: &mut World,
          _: &WorldChannel,
          _: Event<'static, T>,
          _: &EventLoopWindowTarget<T>,
          _: &mut ControlFlow| {}
}
