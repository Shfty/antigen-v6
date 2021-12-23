use std::ops::Deref;

use super::{
    BufferInitDescriptorComponent, BufferWriteComponent, CommandBuffersComponent,
    RenderAttachmentTextureViewDescriptor, SurfaceComponent, SurfaceTextureComponent,
    TextureDescriptorComponent, TextureViewComponent, TextureViewDescriptorComponent,
    TextureWriteComponent, ToBytes,
};
use crate::{
    AdapterComponent, BufferComponent, BufferDescriptorComponent, DeviceComponent,
    InstanceComponent, QueueComponent, RenderAttachmentTextureView, SamplerComponent,
    SamplerDescriptorComponent, ShaderModuleComponent, ShaderModuleDescriptorComponent,
    ShaderModuleDescriptorSpirVComponent, SurfaceConfigurationComponent, TextureComponent,
};

use antigen_core::{Changed, ChangedTrait, Indirect, LazyComponent, Usage};
use antigen_winit::{WindowComponent, WindowEntityMap, WindowEventComponent, WindowSizeComponent};

use hecs::{Entity, World};

use wgpu::{util::DeviceExt, Maintain};

pub fn device_poll_system(maintain: &Maintain) -> impl FnMut(&mut World) {
    let maintain = *maintain;
    move |world| {
        let mut query = world.query::<&DeviceComponent>();
        let (_, device) = query.into_iter().next().unwrap();
        device.poll(maintain);
    }
}

// Initialize pending surfaces that share an entity with a window
pub fn create_window_surfaces_system(world: &mut World) {
    let mut query = world.query::<(
        &WindowComponent,
        &mut SurfaceConfigurationComponent,
        &mut SurfaceComponent,
    )>();
    for (_, (window_component, surface_configuration_component, surface_component)) in
        query.into_iter()
    {
        if let LazyComponent::Ready(window) = &*window_component {
            let mut query = world.query::<&AdapterComponent>();
            let (_, adapter) = query.into_iter().next().unwrap();

            let mut query = world.query::<&DeviceComponent>();
            let (_, device) = query.into_iter().next().unwrap();

            if surface_component.is_pending() {
                let mut query = world.query::<&InstanceComponent>();
                let (_, instance) = query.into_iter().next().unwrap();

                let surface = unsafe { instance.create_surface(window) };

                let window_size = window.inner_size();
                surface_configuration_component.width = window_size.width;
                surface_configuration_component.height = window_size.height;

                surface_configuration_component.format = surface
                    .get_preferred_format(adapter)
                    .expect("Surface is incompatible with adapter");

                surface.configure(device, &*surface_configuration_component);

                surface_component.set_ready(surface);
            }
        }
    }
}

// Initialize pending surfaces that share an entity with a window
pub fn reconfigure_surfaces_system(world: &mut World) {
    let mut query = world.query::<(&SurfaceConfigurationComponent, &SurfaceComponent)>();
    for (_, (surface_config, surface)) in query.into_iter() {
        let mut query = world.query::<&DeviceComponent>();
        let (_, device) = query.into_iter().next().unwrap();

        let surface = if let LazyComponent::Ready(surface) = &*surface {
            surface
        } else {
            return;
        };

        if !surface_config.get_changed() {
            return;
        }

        if surface_config.width > 0 && surface_config.height > 0 {
            surface.configure(device, &surface_config);
        }
    }
}

pub fn reset_surface_config_changed_system(world: &mut World) {
    let mut query = world.query::<&SurfaceConfigurationComponent>();
    for (_, surface_config) in query.into_iter() {
        if surface_config.get_changed() {
            surface_config.set_changed(false);
        }
    }
}

// Fetch the current surface texture for a given surface, and set its dirty flag
pub fn surface_texture_query(world: &mut World, entity: Entity) {
    let mut query = world
        .query_one::<(&SurfaceComponent, &mut SurfaceTextureComponent)>(entity)
        .unwrap();

    let (surface, surface_texture) = if let Some(components) = query.get() {
        components
    } else {
        return;
    };

    let surface = if let LazyComponent::Ready(surface) = &*surface {
        surface
    } else {
        return;
    };

    if let Ok(current) = surface.get_current_texture() {
        **surface_texture = Some(current);
        surface_texture.set_changed(true);
    } else {
        if surface_texture.is_some() {
            surface_texture.set_changed(true);
            **surface_texture = None;
        }
    }
}

// Create a texture view for a surface texture, unsetting its dirty flag
pub fn surface_texture_view_query(world: &mut World, entity: Entity) {
    let mut query = world
        .query_one::<(
            &SurfaceTextureComponent,
            &RenderAttachmentTextureViewDescriptor<'static>,
            &mut RenderAttachmentTextureView,
        )>(entity)
        .unwrap();

    let (surface_texture_component, texture_view_desc, texture_view) =
        if let Some(components) = query.get() {
            components
        } else {
            return;
        };

    if surface_texture_component.get_changed() {
        if let Some(surface_texture) = &**surface_texture_component {
            let view = surface_texture.texture.create_view(&texture_view_desc);
            texture_view.set_ready(view);
            surface_texture_component.set_changed(false);
        } else {
            texture_view.set_dropped();
            surface_texture_component.set_changed(false);
        }
    }
}

pub fn surface_size_system(world: &mut World) {
    let mut query = world.query::<(&WindowSizeComponent, &mut SurfaceConfigurationComponent)>();
    for (_, (window_size, surface_configuration)) in query.into_iter() {
        if window_size.get_changed() {
            surface_configuration.width = window_size.width;
            surface_configuration.height = window_size.height;
            surface_configuration.set_changed(true);
        }
    }
}

// Present valid surface textures, setting their dirty flag
pub fn surface_texture_present_system(world: &mut World) {
    let mut query = world.query::<&mut SurfaceTextureComponent>();
    for (_, surface_texture_component) in query.into_iter() {
        if let Some(surface_texture) = surface_texture_component.take() {
            println!("Presenting surface texture {:?}", surface_texture);
            surface_texture.present();
            surface_texture_component.set_changed(true);
        }
    }
}

// Drop texture views whose surface textures have been invalidated, unsetting their dirty flag
pub fn surface_texture_view_drop_system(world: &mut World) {
    let mut query = world.query::<(
        &mut SurfaceTextureComponent,
        &mut RenderAttachmentTextureView,
    )>();
    for (_, (surface_texture, texture_view)) in query.into_iter() {
        if !surface_texture.get_changed() {
            return;
        }

        if surface_texture.is_some() {
            return;
        }

        println!("Dropping texture view for surface texture {:?}", surface_texture);
        texture_view.set_dropped();
        surface_texture.set_changed(false);
    }
}

/// Create pending usage-tagged shader modules, recreating them if a Changed flag is set
pub fn create_shader_modules_system(world: &mut World) {
    println!("Create shader modules system");
    let mut query = world.query::<(
        &ShaderModuleDescriptorComponent,
        &mut ShaderModuleComponent,
    )>();

    for (entity, (shader_module_desc, shader_module)) in query.into_iter() {
        println!("Checking shader for entity {:?}", entity);
        if !shader_module.is_pending() && !shader_module_desc.get_changed() {
            continue;
        }

        let mut query = world.query::<&DeviceComponent>();
        let (_, device) = query.into_iter().next().unwrap();
        shader_module.set_ready(device.create_shader_module(&shader_module_desc));

        shader_module_desc.set_changed(false);
        println!("Created shader module with label {:?}", shader_module_desc.label);
    }
}

/// Create pending usage-tagged shader modules, recreating them if a Changed flag is set
pub fn create_shader_modules_spirv_system<T: Send + Sync + 'static>(world: &mut World) {
    let mut query = world.query::<(
        &Usage<T, ShaderModuleDescriptorSpirVComponent>,
        &mut Usage<T, ShaderModuleComponent>,
    )>();
    for (_, (shader_module_desc, shader_module)) in query.into_iter() {
        if !shader_module.is_pending() && !shader_module_desc.get_changed() {
            return;
        }

        let mut query = world.query::<&DeviceComponent>();
        let (_, device) = query.into_iter().next().unwrap();
        shader_module.set_ready(unsafe { device.create_shader_module_spirv(&shader_module_desc) });

        shader_module_desc.set_changed(false);
        println!(
            "Created {} spir-v shader module",
            std::any::type_name::<T>()
        );
    }
}

/// Create pending usage-tagged buffers, recreating them if a Changed flag is set
pub fn create_buffers_system<T: Send + Sync + 'static>(world: &mut World) {
    let mut query = world.query::<(
        &Usage<T, BufferDescriptorComponent>,
        &mut Usage<T, BufferComponent>,
    )>();

    for (_, (buffer_descriptor, buffer)) in query.into_iter() {
        if !buffer.is_pending() && !buffer_descriptor.get_changed() {
            return;
        }

        let mut query = world.query::<&DeviceComponent>();
        let (_, device) = query.iter().next().unwrap();
        buffer.set_ready(device.create_buffer(&buffer_descriptor));

        buffer_descriptor.set_changed(false);

        println!("Created {} buffer", std::any::type_name::<T>());
    }
}

/// Create-initialize pending usage-tagged buffers, recreating them if a Changed flag is set
pub fn create_buffers_init_system<T: Send + Sync + 'static>(world: &mut World) {
    let mut query = world.query::<(
        &Usage<T, BufferInitDescriptorComponent>,
        &mut Usage<T, BufferComponent>,
    )>();

    for (_, (buffer_init_descriptor, buffer)) in query.into_iter() {
        if !buffer.is_pending() && !buffer_init_descriptor.get_changed() {
            return;
        }

        let mut query = world.query::<&DeviceComponent>();
        let (_, device) = query.into_iter().next().unwrap();
        buffer.set_ready(device.create_buffer_init(&buffer_init_descriptor));

        buffer_init_descriptor.set_changed(false);

        println!("Create-initialized {} buffer", std::any::type_name::<T>());
    }
}

/// Create pending usage-tagged textures, recreating them if a Changed flag is set
pub fn create_textures_system<T: Send + Sync + 'static>(world: &mut World) {
    let mut query = world.query::<(
        &Usage<T, TextureDescriptorComponent>,
        &mut Usage<T, TextureComponent>,
    )>();

    for (_, (texture_descriptor_component, texture)) in query.into_iter() {
        if !texture.is_pending() && !texture_descriptor_component.get_changed() {
            return;
        }

        let texture_descriptor = texture_descriptor_component;
        if texture_descriptor.size.width == 0
            || texture_descriptor.size.height == 0
            || texture_descriptor.size.depth_or_array_layers == 0
        {
            return;
        }

        let mut query = world.query::<&DeviceComponent>();
        let (_, device) = query.into_iter().next().unwrap();

        texture.set_ready(device.create_texture(&*texture_descriptor));

        texture_descriptor_component.set_changed(false);

        println!("Created texture: {:#?}", ***texture_descriptor);
    }
}

/// Create pending usage-tagged texture views, recreating them if a Changed flag is set
pub fn create_texture_views_system<T: Send + Sync + 'static>(world: &mut World) {
    let mut query = world.query::<(
        &Usage<T, TextureComponent>,
        &Usage<T, TextureViewDescriptorComponent<'static>>,
        &mut Usage<T, TextureViewComponent>,
    )>();

    for (_, (texture, texture_view_descriptor, texture_view)) in query.into_iter() {
        if !texture_view.is_pending() && !texture_view_descriptor.get_changed() {
            return;
        }

        let texture = if let LazyComponent::Ready(texture) = &**texture {
            texture
        } else {
            return;
        };

        texture_view.set_ready(texture.create_view(&texture_view_descriptor));

        texture_view_descriptor.set_changed(false);

        println!("Created texture view: {:#?}", ***texture_view_descriptor);
    }
}

/// Create pending usage-tagged samplers, recreating them if a Changed flag is set
pub fn create_samplers_system<T: Send + Sync + 'static>(world: &mut World) {
    let mut query = world.query::<(
        &Usage<T, SamplerDescriptorComponent>,
        &mut Usage<T, SamplerComponent>,
    )>();

    for (_, (sampler_descriptor, sampler)) in query.into_iter() {
        if !sampler.is_pending() && !sampler_descriptor.get_changed() {
            return;
        }

        let mut query = world.query::<&DeviceComponent>();
        let (_, device) = query.into_iter().next().unwrap();
        sampler.set_ready(device.create_sampler(&sampler_descriptor));

        sampler_descriptor.set_changed(false);

        println!("Created sampler: {:#?}", ***sampler_descriptor);
    }
}

// Write data to buffer
pub fn buffer_write_system<
    T: Send + Sync + 'static,
    L: Deref<Target = V> + Send + Sync + 'static,
    V: ToBytes,
>(
    world: &mut World,
) {
    let mut query = world.query::<&QueueComponent>();
    let (_, queue) = if let Some(components) = query.into_iter().next() {
        components
    } else {
        return;
    };

    let mut query = world.query::<(
        &Usage<T, BufferWriteComponent<L>>,
        &Changed<L>,
        &Indirect<Usage<T, BufferComponent>>,
    )>();

    for (_, (buffer_write, data_component, buffer)) in query.into_iter() {
        let mut query = buffer.get(world);
        let buffer = query.get().unwrap_or_else(|| panic!("No buffer component of type {}", std::any::type_name::<T>()));


        if data_component.get_changed() {
            let buffer = if let LazyComponent::Ready(buffer) = &**buffer {
                buffer
            } else {
                return;
            };

            let bytes = data_component.to_bytes();

            println!(
                "Writing {} bytes to {} buffer at offset {}",
                bytes.len(),
                std::any::type_name::<T>(),
                buffer_write.offset(),
            );
            queue.write_buffer(buffer, buffer_write.offset(), bytes);

            data_component.set_changed(false);
        }
    }
}

// Write data to texture
pub fn texture_write_system<T, L, V>(world: &mut World)
where
    T: Send + Sync + 'static,
    L: Deref<Target = V> + Send + Sync + 'static,
    V: ToBytes,
{
    let mut query = world.query::<&QueueComponent>();
    let (_, queue) = if let Some(queue) = query.into_iter().next() {
        queue
    } else {
        return;
    };

    let mut query = world.query::<(
        &Usage<T, TextureWriteComponent<L>>,
        &Changed<L>,
        &Usage<T, TextureDescriptorComponent>,
        &Usage<T, TextureComponent>,
    )>();

    for (_, (texture_write, texels_component, texture_descriptor_component, texture_component)) in
        query.into_iter()
    {
        if texels_component.get_changed() {
            let texture = if let LazyComponent::Ready(texture) = &**texture_component {
                texture
            } else {
                return;
            };

            let bytes = texels_component.to_bytes();
            let image_copy_texture = texture_write.image_copy_texture();
            let image_data_layout = texture_write.image_data_layout();

            println!(
                "Writing {} bytes to texture at offset {}",
                bytes.len(),
                image_data_layout.offset,
            );

            queue.write_texture(
                wgpu::ImageCopyTexture {
                    texture: &*texture,
                    mip_level: image_copy_texture.mip_level,
                    origin: image_copy_texture.origin,
                    aspect: image_copy_texture.aspect,
                },
                bytes,
                *image_data_layout,
                texture_descriptor_component.size,
            );

            texels_component.set_changed(false);
        }
    }
}

// Flush command buffers to the WGPU queue
pub fn submit_command_buffers_system(world: &mut World) {
    let mut query = world.query::<&mut CommandBuffersComponent>();

    for (_, command_buffers) in query.into_iter() {
        let mut query = world.query::<&QueueComponent>();
        let (_, queue) = if let Some(queue) = query.into_iter().next() {
            queue
        } else {
            return;
        };

        println!("Submitting command buffers: {:?}", command_buffers);
        queue.submit(command_buffers.drain(..));
    }
}

// Create textures and corresponding texture views for surfaces
pub fn surfaces_textures_views_system(world: &mut World) {
    let mut query = world.query::<&WindowEventComponent>();
    let (_, window_event) = query.into_iter().next().unwrap();
    let window_event = window_event.0;
    drop(query);

    let window_event = window_event.expect("No window for current event");

    let mut query = world.query::<&WindowEntityMap>();

    let (_, window_entity_map) = query.into_iter().next().unwrap();

    let entity = *window_entity_map
        .get(&window_event)
        .expect("Redraw requested for window without entity");

    drop(query);

    // Create surface textures and views
    // These will be rendered to and presented during RedrawEventsCleared
    surface_texture_query(world, entity);
    surface_texture_view_query(world, entity);
}
