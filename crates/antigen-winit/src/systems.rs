use super::{RedrawUnconditionally, WindowComponent};
use crate::{WindowEntityMap, WindowEventComponent, WindowSizeComponent, WindowTitleComponent};
use hecs::World;

use antigen_core::{ChangedTrait, LazyComponent};

use winit::event_loop::EventLoopWindowTarget;

// Create winit::Window for WindowComponent
pub fn create_windows_system<T>(world: &mut World, event_loop_proxy: &EventLoopWindowTarget<T>) {
    let mut query = world.query::<&mut WindowEntityMap>();
    let (_, window_entity_map) = query.into_iter().next().unwrap();

    let mut query = world.query::<&WindowComponent>();
    let pending_entities = query
        .into_iter()
        .flat_map(|(entity, window_component)| match *window_component {
            LazyComponent::Pending => Some(entity),
            _ => None,
        })
        .collect::<Vec<_>>();
    drop(query);

    for entity in pending_entities {
        let mut query = world
            .query_one::<(&mut WindowComponent, Option<&mut WindowSizeComponent>)>(entity)
            .unwrap();

        let (window_component, size_component) = query.get().unwrap();

        let window = winit::window::Window::new(event_loop_proxy).unwrap();
        let size = window.inner_size();

        window_entity_map.insert(window.id(), entity);
        window_component.set_ready(window);

        if let Some(window_size) = size_component {
            ***window_size = size;
            window_size.set_changed(true);
        }
    }
}

// Request redraws for WindowComponents
pub fn redraw_unconditionally_system(world: &mut World) {
    for (_, window) in world
        .query_mut::<&WindowComponent>()
        .with::<RedrawUnconditionally>()
        .into_iter()
    {
        match &*window {
            LazyComponent::Ready(window) => window.request_redraw(),
            _ => (),
        }
    }
}

pub fn resize_window_system(world: &mut World) {
    let mut query = world.query::<&WindowEventComponent>();
    let (_, event_window) = query.into_iter().next().unwrap();

    let window_id = event_window.0.expect("No window for current event");

    let mut query = world.query::<&WindowEntityMap>();
    let (_, window_entity_map) = query.into_iter().next().unwrap();

    let entity = window_entity_map
        .get(&window_id)
        .expect("Resize requested for window without entity");

    let mut query = world
        .query_one::<(&WindowComponent, &mut WindowSizeComponent)>(*entity)
        .unwrap();

    let (window_component, size_component) = if let Some(components) = query.get() {
        components
    } else {
        return;
    };

    if let LazyComponent::Ready(window) = &*window_component {
        ***size_component = window.inner_size();
        size_component.set_changed(true);
    }
}

pub fn reset_window_size_changed_system(world: &mut World) {
    for (_, window_size) in world.query_mut::<&mut WindowSizeComponent>() {
        if window_size.get_changed() {
            println!("Resetting window size changed flag");
            window_size.set_changed(false);
        }
    }
}

pub fn window_title_system(world: &mut World) {
    world
        .query_mut::<(&WindowComponent, &WindowTitleComponent)>()
        .into_iter()
        .for_each(|(_, (window, title))| {
            if let LazyComponent::Ready(window) = &*window {
                if title.get_changed() {
                    window.set_title(&title);
                    title.set_changed(false);
                }
            }
        });
}

pub fn close_window_system(world: &mut World) {
    let mut query = world.query::<&WindowEventComponent>();
    let (_, window_event) = query.into_iter().next().unwrap();

    let window_id = if let (Some(window_id), _) = &*window_event {
        window_id
    } else {
        return;
    };

    let mut query = world.query::<&WindowEntityMap>();
    let (_, window_entity_map) = query.into_iter().next().unwrap();

    let entity = window_entity_map
        .get(&window_id)
        .expect("Close requested for window without entity");

    let mut query = world.query_one::<&mut WindowComponent>(*entity).unwrap();
    let window_component = query.get().unwrap();
    if window_component.is_ready() {
        window_component.set_dropped()
    } else {
        panic!("Close requested for a non-open window");
    }
}
