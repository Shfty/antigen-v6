use std::{sync::atomic::Ordering, time::Instant};

use super::*;
use antigen_core::{
    get_named_entities_component, Changed, ChangedTrait, CopyToComponent, Indirect, LazyComponent,
    NamedEntitiesComponent,
};

use antigen_wgpu::{
    wgpu::{
        BindGroupDescriptor, BindGroupEntry, BindGroupLayoutDescriptor, BindGroupLayoutEntry,
        BindingResource, BindingType, BufferBinding, BufferBindingType, BufferSize, Extent3d,
        ShaderStages,
    },
    BindGroupComponent, BindGroupLayoutComponent, BufferComponent, DeviceComponent,
    RenderPassDrawComponent, SamplerComponent, SurfaceConfigurationComponent,
    TextureDescriptorComponent, TextureViewComponent, TextureViewDescriptorComponent,
};

use hecs::World;
use winit::event::{ElementState, KeyboardInput};

// Initialize the hello triangle render pipeline
pub fn phosphor_prepare_system(world: &mut World) {
    // Fetch resources
    let mut query = world.query::<&DeviceComponent>();
    let (_, device) = query.into_iter().next().unwrap();

    let mut query = world.query::<&PhosphorRenderer>();
    for (entity, _) in query.into_iter() {
        phosphor_prepare(world, entity, device);
    }
}

pub fn phosphor_prepare_uniform_bind_group(
    device: &DeviceComponent,
    uniform_buffer: &BufferComponent,
    uniform_bind_group_layout: &mut BindGroupLayoutComponent,
    uniform_bind_group: &mut BindGroupComponent,
) -> Option<()> {
    let uniform_buffer = uniform_buffer.read();
    let uniform_buffer = uniform_buffer.get()?;

    // Uniform bind group
    let uniform_bind_group_layout = if let Some(bind_group_layout) = uniform_bind_group_layout.get()
    {
        bind_group_layout
    } else {
        let bind_group_layout = device.create_bind_group_layout(&BindGroupLayoutDescriptor {
            label: Some("Uniform Bind Group Layout"),
            entries: &[BindGroupLayoutEntry {
                binding: 0,
                visibility: ShaderStages::VERTEX | ShaderStages::FRAGMENT,
                ty: BindingType::Buffer {
                    ty: BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: BufferSize::new(176),
                },
                count: None,
            }],
        });

        uniform_bind_group_layout.set_ready_with(bind_group_layout);
        uniform_bind_group_layout.get().unwrap()
    };

    if uniform_bind_group.is_pending() {
        let bind_group = device.create_bind_group(&BindGroupDescriptor {
            layout: &uniform_bind_group_layout,
            entries: &[BindGroupEntry {
                binding: 0,
                resource: uniform_buffer.as_entire_binding(),
            }],
            label: None,
        });

        uniform_bind_group.set_ready_with(bind_group);
    }

    Some(())
}

pub fn phosphor_prepare_storage_bind_group(
    device: &DeviceComponent,
    vertex_buffer: &BufferComponent,
    triangle_mesh_instance_buffer: &BufferComponent,
    line_index_buffer: &BufferComponent,
    line_mesh_buffer: &BufferComponent,
    line_mesh_instance_buffer: &BufferComponent,
    line_instance_buffer: &BufferComponent,
    bind_group_layout: &mut BindGroupLayoutComponent,
    bind_group: &mut BindGroupComponent,
) -> Option<()> {
    let vertex_buffer = vertex_buffer.read();
    let vertex_buffer = vertex_buffer.get()?;

    let triangle_mesh_instance_buffer = triangle_mesh_instance_buffer.read();
    let triangle_mesh_instance_buffer = triangle_mesh_instance_buffer.get()?;

    let line_index_buffer = line_index_buffer.read();
    let line_index_buffer = line_index_buffer.get()?;

    let line_mesh_buffer = line_mesh_buffer.read();
    let line_mesh_buffer = line_mesh_buffer.get()?;

    let line_mesh_instance_buffer = line_mesh_instance_buffer.read();
    let line_mesh_instance_buffer = line_mesh_instance_buffer.get()?;

    let line_instance_buffer = line_instance_buffer.read();
    let line_instance_buffer = line_instance_buffer.get()?;

    let bind_group_layout = match bind_group_layout.get() {
        Some(bind_group_layout) => bind_group_layout,
        None => {
            let storage_bind_group_layout =
                device.create_bind_group_layout(&BindGroupLayoutDescriptor {
                    label: Some("Storage Buffer Bind Group Layout"),
                    entries: &[
                        BindGroupLayoutEntry {
                            binding: 0,
                            visibility: ShaderStages::VERTEX,
                            ty: BindingType::Buffer {
                                ty: BufferBindingType::Storage { read_only: true },
                                has_dynamic_offset: false,
                                min_binding_size: BufferSize::new(48),
                            },
                            count: None,
                        },
                        BindGroupLayoutEntry {
                            binding: 1,
                            visibility: ShaderStages::VERTEX,
                            ty: BindingType::Buffer {
                                ty: BufferBindingType::Storage { read_only: true },
                                has_dynamic_offset: true,
                                min_binding_size: BufferSize::new(
                                    buffer_size_of::<TriangleMeshInstanceData>()
                                        * MAX_TRIANGLE_MESH_INSTANCES as BufferAddress,
                                ),
                            },
                            count: None,
                        },
                        BindGroupLayoutEntry {
                            binding: 2,
                            visibility: ShaderStages::VERTEX,
                            ty: BindingType::Buffer {
                                ty: BufferBindingType::Storage { read_only: true },
                                has_dynamic_offset: false,
                                min_binding_size: BufferSize::new(4),
                            },
                            count: None,
                        },
                        BindGroupLayoutEntry {
                            binding: 3,
                            visibility: ShaderStages::VERTEX,
                            ty: BindingType::Buffer {
                                ty: BufferBindingType::Storage { read_only: true },
                                has_dynamic_offset: false,
                                min_binding_size: BufferSize::new(16),
                            },
                            count: None,
                        },
                        BindGroupLayoutEntry {
                            binding: 4,
                            visibility: ShaderStages::VERTEX,
                            ty: BindingType::Buffer {
                                ty: BufferBindingType::Storage { read_only: true },
                                has_dynamic_offset: false,
                                min_binding_size: BufferSize::new(48),
                            },
                            count: None,
                        },
                        BindGroupLayoutEntry {
                            binding: 5,
                            visibility: ShaderStages::VERTEX,
                            ty: BindingType::Buffer {
                                ty: BufferBindingType::Storage { read_only: true },
                                has_dynamic_offset: false,
                                min_binding_size: BufferSize::new(8),
                            },
                            count: None,
                        },
                    ],
                });

            bind_group_layout.set_ready_with(storage_bind_group_layout);
            bind_group_layout.get().unwrap()
        }
    };

    if bind_group.is_pending() {
        let storage_bind_group = device.create_bind_group(&BindGroupDescriptor {
            layout: &bind_group_layout,
            entries: &[
                BindGroupEntry {
                    binding: 0,
                    resource: vertex_buffer.as_entire_binding(),
                },
                BindGroupEntry {
                    binding: 1,
                    resource: BindingResource::Buffer(BufferBinding {
                        buffer: triangle_mesh_instance_buffer,
                        offset: 0,
                        size: BufferSize::new(
                            buffer_size_of::<TriangleMeshInstanceData>()
                                * MAX_TRIANGLE_MESH_INSTANCES as BufferAddress,
                        ),
                    }),
                },
                BindGroupEntry {
                    binding: 2,
                    resource: line_index_buffer.as_entire_binding(),
                },
                BindGroupEntry {
                    binding: 3,
                    resource: line_mesh_buffer.as_entire_binding(),
                },
                BindGroupEntry {
                    binding: 4,
                    resource: line_mesh_instance_buffer.as_entire_binding(),
                },
                BindGroupEntry {
                    binding: 5,
                    resource: line_instance_buffer.as_entire_binding(),
                },
            ],
            label: None,
        });

        bind_group.set_ready_with(storage_bind_group);
    }

    Some(())
}

pub fn phosphor_prepare(world: &World, entity: Entity, device: &DeviceComponent) -> Option<()> {
    let mut query = world.query_one::<&SamplerComponent>(entity).unwrap();
    let sampler = query.get().unwrap();

    let mut query = world
        .query_one::<&Indirect<&SurfaceConfigurationComponent>>(entity)
        .unwrap();
    let mut query = query.get().unwrap().get(world);
    let surface_config = query.get().unwrap();

    let mut query = world
        .query::<(
            &BufferComponent,
            &mut BindGroupLayoutComponent,
            &mut BindGroupComponent,
        )>()
        .with::<Uniform>();
    let (_, (uniform_buffer, uniform_bind_group_layout, uniform_bind_group)) =
        query.into_iter().next()?;

    let mut query = world.query::<(&BufferComponent,)>().with::<Vertices>();
    let (_, (vertex_buffer,)) = query.into_iter().next()?;

    let mut query = world
        .query::<(&BufferComponent,)>()
        .with::<TriangleMeshInstances>();
    let (_, (triangle_mesh_instance_buffer,)) = query.into_iter().next()?;

    let mut query = world.query::<(&BufferComponent,)>().with::<LineIndices>();
    let (_, (line_index_buffer,)) = query.into_iter().next()?;

    let mut query = world.query::<(&BufferComponent,)>().with::<LineMeshes>();
    let (_, (line_mesh_buffer,)) = query.into_iter().next()?;

    let mut query = world
        .query::<(&BufferComponent,)>()
        .with::<LineMeshInstances>();
    let (_, (line_mesh_instance_buffer,)) = query.into_iter().next()?;

    let mut query = world.query::<(&BufferComponent,)>().with::<LineInstances>();
    let (_, (line_instance_buffer,)) = query.into_iter().next()?;

    let mut query = world
        .query::<(&TextureViewComponent,)>()
        .with::<BeamBuffer>();
    let (_, (beam_buffer_view,)) = query.into_iter().next()?;

    let mut query = world
        .query::<(&TextureViewComponent,)>()
        .with::<PhosphorFrontBuffer>();
    let (_, (phosphor_front_buffer_view,)) = query.into_iter().next()?;

    let mut query = world
        .query::<(&TextureViewComponent,)>()
        .with::<PhosphorBackBuffer>();
    let (_, (phosphor_back_buffer_view,)) = query.into_iter().next()?;

    phosphor_prepare_uniform_bind_group(
        device,
        uniform_buffer,
        uniform_bind_group_layout,
        uniform_bind_group,
    );

    let mut query = world
        .query::<(&mut BindGroupLayoutComponent, &mut BindGroupComponent)>()
        .with::<StorageBuffers>();
    let (_, (storage_bind_group_layout, storage_bind_group)) = query.into_iter().next()?;
    println!("Fetched storage bind group entity");

    phosphor_prepare_storage_bind_group(
        device,
        vertex_buffer,
        triangle_mesh_instance_buffer,
        line_index_buffer,
        line_mesh_buffer,
        line_mesh_instance_buffer,
        line_instance_buffer,
        storage_bind_group_layout,
        storage_bind_group,
    )?;

    let mut query = world.query::<&ShaderModuleComponent>().with::<Beam>();

    let (_, beam_shader) = query.into_iter().next()?;
    println!("Fetched beam shader entity");

    let mut query = world
        .query::<&mut RenderPipelineComponent>()
        .with::<BeamClear>();

    let (_, beam_clear_pipeline) = query.into_iter().next()?;
    println!("Fetched beam clear pass entity");

    phosphor_prepare_beam_clear(device, beam_shader, beam_clear_pipeline)?;

    let mut query = world
        .query::<&mut RenderPipelineComponent>()
        .with::<BeamTriangles>();

    let (_, beam_mesh_pipeline) = query.into_iter().next()?;
    println!("Fetched beam mesh pass entity");

    phosphor_prepare_beam_mesh(
        device,
        uniform_bind_group_layout,
        storage_bind_group_layout,
        beam_shader,
        beam_mesh_pipeline,
    )?;

    let mut query = world
        .query::<&mut RenderPipelineComponent>()
        .with::<BeamLines>();
    let (_, beam_line_pipeline) = query.into_iter().next()?;
    println!("Fetched beam line pass entity");

    phosphor_prepare_beam_line(
        device,
        uniform_bind_group_layout,
        storage_bind_group_layout,
        beam_shader,
        beam_line_pipeline,
    )?;

    let mut query = world
        .query::<(
            &ShaderModuleComponent,
            &mut RenderPipelineComponent,
            &mut BindGroupLayoutComponent,
        )>()
        .with::<PhosphorDecay>();
    let (_, (phosphor_decay_shader, phosphor_decay_pipeline, phosphor_bind_group_layout)) =
        query.into_iter().next()?;
    println!("Fetched phosphor decay pass entity");

    let mut query = world
        .query::<(&mut BindGroupComponent,)>()
        .with::<PhosphorFrontBuffer>();
    let (_, (front_bind_group,)) = query.into_iter().next()?;

    let mut query = world
        .query::<(&mut BindGroupComponent,)>()
        .with::<PhosphorBackBuffer>();
    let (_, (back_bind_group,)) = query.into_iter().next()?;

    phosphor_prepare_phosphor_decay(
        device,
        phosphor_bind_group_layout,
        front_bind_group,
        back_bind_group,
        phosphor_decay_pipeline,
        uniform_bind_group_layout,
        phosphor_decay_shader,
        sampler,
        beam_buffer_view,
        phosphor_front_buffer_view,
        phosphor_back_buffer_view,
    )?;

    let mut query = world
        .query::<(&ShaderModuleComponent, &mut RenderPipelineComponent)>()
        .with::<Tonemap>();
    let (_, (tonemap_shader, tonemap_pipeline)) = query.into_iter().next()?;
    println!("Fetched tonemap pass entity");

    phosphor_prepare_tonemap(
        device,
        phosphor_bind_group_layout,
        tonemap_shader,
        surface_config,
        tonemap_pipeline,
    )?;

    Some(())
}

// Game tick update
pub fn phosphor_update_total_time_system(world: &mut World) {
    for (_, (start_time, total_time)) in
        world.query_mut::<(&StartTimeComponent, &mut Changed<TotalTimeComponent>)>()
    {
        ***total_time = Instant::now().duration_since(**start_time).as_secs_f32();
        println!("Total time: {:#?}", ***total_time);
        total_time.set_changed(true);
    }
}

pub fn phosphor_update_delta_time_system(world: &mut World) {
    for (_, (timestamp, delta_time)) in
        world.query_mut::<(&TimestampComponent, &mut Changed<DeltaTimeComponent>)>()
    {
        let timestamp = **timestamp;
        ***delta_time = Instant::now().duration_since(timestamp).as_secs_f32();
        println!("Delta time: {:#?}", ***delta_time);
        delta_time.set_changed(true);
    }
}

pub fn phosphor_update_timestamp_system(world: &mut World) {
    for (_, timestamp) in world.query_mut::<&mut TimestampComponent>() {
        **timestamp = Instant::now();
    }
}

pub fn phosphor_update_timers_system(world: &mut World) {
    for (_, timer) in world.query_mut::<&mut TimerComponent>() {
        let now = Instant::now();
        if now.duration_since(timer.timestamp) > timer.duration {
            timer.timestamp = now;
            timer.set_changed(true);
        }
    }
}

pub fn phosphor_update_oscilloscopes_system(world: &mut World) {
    println!("Update oscilloscopes system");
    let mut query = world.query::<&Changed<TotalTimeComponent>>();
    let (_, total_time) = query.iter().next().expect("No total time component");

    let mut query = world.query::<&Changed<DeltaTimeComponent>>();
    let (_, delta_time) = query.iter().next().expect("No delta time component");

    for (_, (oscilloscope, vertex_data)) in world
        .query::<(&Oscilloscope, &mut Changed<VertexDataComponent>)>()
        .into_iter()
    {
        let (fx, fy, fz) = oscilloscope.eval(***total_time);

        for i in 1..vertex_data.len() {
            let i0 = i - 1;
            let i1 = i;
            vertex_data[i0] = vertex_data[i1];
            vertex_data[i0].intensity += vertex_data[i0].delta_intensity * ***delta_time;
        }

        let last_idx = vertex_data.len() - 1;
        let last = &mut vertex_data[last_idx];
        last.position[0] = fx;
        last.position[1] = fy;
        last.position[2] = fz;

        vertex_data.set_changed(true);
    }
}

pub fn phosphor_resize_system(world: &mut World) {
    let mut query = world
        .query::<&Indirect<&SurfaceConfigurationComponent>>()
        .with::<PhosphorRenderer>();
    let (_, indirect) = query.into_iter().next().unwrap();

    let mut query = indirect.get(world);
    let surface_config = query.get().unwrap();

    if !surface_config.get_changed() {
        return;
    }

    let extent = Extent3d {
        width: surface_config.width,
        height: surface_config.height,
        depth_or_array_layers: 1,
    };

    let mut query = world
        .query::<(&mut BindGroupComponent,)>()
        .with::<PhosphorFrontBuffer>();
    let (_, (front_bind_group,)) = query.into_iter().next().unwrap();

    let mut query = world
        .query::<(&mut BindGroupComponent,)>()
        .with::<PhosphorBackBuffer>();
    let (_, (back_bind_group,)) = query.into_iter().next().unwrap();

    let mut query = world
        .query::<(&mut Changed<PerspectiveMatrixComponent>,)>()
        .with::<PerspectiveMatrix>();
    let (_, (perspective_matrix,)) = query.into_iter().next().unwrap();

    let mut query = world
        .query::<(&mut Changed<OrthographicMatrixComponent>,)>()
        .with::<OrthographicMatrix>();
    let (_, (orthographic_matrix,)) = query.into_iter().next().unwrap();

    let mut query = world
        .query::<(
            &mut TextureDescriptorComponent,
            &mut TextureViewDescriptorComponent,
        )>()
        .with::<BeamBuffer>();
    let (_, (beam_buffer_desc, beam_buffer_view_desc)) = query.into_iter().next().unwrap();

    let mut query = world
        .query::<(
            &mut TextureDescriptorComponent,
            &mut TextureViewDescriptorComponent,
        )>()
        .with::<BeamDepthBuffer>();
    let (_, (beam_depth_desc, beam_depth_view_desc)) = query.into_iter().next().unwrap();

    let mut query = world
        .query::<(
            &mut TextureDescriptorComponent,
            &mut TextureViewDescriptorComponent,
        )>()
        .with::<BeamMultisample>();
    let (_, (beam_multisample_desc, beam_multisample_view_desc)) =
        query.into_iter().next().unwrap();

    let mut query = world
        .query::<(
            &mut TextureDescriptorComponent,
            &mut TextureViewDescriptorComponent,
        )>()
        .with::<PhosphorFrontBuffer>();
    let (_, (phosphor_front_desc, phosphor_front_view_desc)) = query.into_iter().next().unwrap();

    let mut query = world
        .query::<(
            &mut TextureDescriptorComponent,
            &mut TextureViewDescriptorComponent,
        )>()
        .with::<PhosphorBackBuffer>();
    let (_, (phosphor_back_desc, phosphor_back_view_desc)) = query.into_iter().next().unwrap();

    beam_buffer_desc.size = extent;
    beam_depth_desc.size = extent;
    beam_multisample_desc.size = extent;
    phosphor_front_desc.size = extent;
    phosphor_back_desc.size = extent;

    beam_buffer_desc.set_changed(true);
    beam_depth_desc.set_changed(true);
    beam_multisample_desc.set_changed(true);
    phosphor_front_desc.set_changed(true);
    phosphor_back_desc.set_changed(true);

    beam_buffer_view_desc.set_changed(true);
    beam_depth_view_desc.set_changed(true);
    beam_multisample_view_desc.set_changed(true);
    phosphor_front_view_desc.set_changed(true);
    phosphor_back_view_desc.set_changed(true);

    front_bind_group.set_pending();
    back_bind_group.set_pending();

    let aspect = surface_config.width as f32 / surface_config.height as f32;

    ***perspective_matrix = super::perspective_matrix(aspect, NEAR_PLANE);
    perspective_matrix.set_changed(true);

    ***orthographic_matrix = super::orthographic_matrix(aspect, 200.0);
    orthographic_matrix.set_changed(true);
}

pub fn phosphor_mouse_moved_system(world: &mut World, (delta_x, delta_y): (f64, f64)) {
    let mut query = world
        .query::<(&mut EulerAnglesComponent, &mut Changed<RotationComponent>)>()
        .with::<Camera>();
    let (_, (euler_angles, rotation)) = query.into_iter().next().unwrap();

    euler_angles.y += delta_x as f32 * 0.004;
    euler_angles.x += delta_y as f32 * 0.004;

    let pitch = nalgebra::UnitQuaternion::from_euler_angles(euler_angles.x, 0.0, 0.0);
    let yaw = nalgebra::UnitQuaternion::from_euler_angles(0.0, euler_angles.y, 0.0);
    let quat = pitch * yaw;

    ***rotation = quat.into();
    rotation.set_changed(true);
}

pub fn phosphor_key_event_system(world: &mut World, key_event: KeyboardInput) {
    let (_, player_input) = world
        .query_mut::<&mut PlayerInputComponent>()
        .into_iter()
        .next()
        .unwrap();

    let key_value = || match key_event.state {
        ElementState::Pressed => 1.0,
        ElementState::Released => 0.0,
    };

    match key_event.virtual_keycode {
        Some(key) => match key {
            winit::event::VirtualKeyCode::D => player_input.back = key_value(),
            winit::event::VirtualKeyCode::E => player_input.forward = key_value(),
            winit::event::VirtualKeyCode::F => player_input.right = key_value(),
            winit::event::VirtualKeyCode::S => player_input.left = key_value(),
            winit::event::VirtualKeyCode::W => player_input.down = key_value(),
            winit::event::VirtualKeyCode::R => player_input.up = key_value(),
            _ => (),
        },
        _ => (),
    }
}

pub fn phosphor_camera_position_system(world: &mut World) {
    // Get player input
    let mut query = world.query::<&mut PlayerInputComponent>();
    let (_, player_input) = query.into_iter().next().unwrap();

    // Get camera entity
    let mut query = world
        .query::<(
            &mut Changed<PositionComponent>,
            &mut Changed<RotationComponent>,
        )>()
        .with::<Camera>();
    let (_, (position, rotation)) = query.into_iter().next().unwrap();

    let mut delta = nalgebra::Vector3::<f32>::default();

    delta.x += player_input.right;
    delta.x -= player_input.left;
    delta.z -= player_input.forward;
    delta.z += player_input.back;
    delta.y += player_input.up;
    delta.y -= player_input.down;

    ***position += rotation.conjugate() * delta;

    position.set_changed(true);
}

pub fn phosphor_update_beam_mesh_draw_count_system(world: &mut World) {
    let mut query = world
        .query::<&antigen_wgpu::BufferLengthsComponent>()
        .with::<TriangleMeshInstances>();
    let (_, mesh_instance_counts) = query.into_iter().next().unwrap();

    let mut query = world
        .query::<&mut Changed<TriangleMeshDataComponent>>()
        .with::<BeamTriangles>();

    for (i, (_, triangle_mesh_data)) in query.into_iter().enumerate() {
        triangle_mesh_data[0].instance_count = mesh_instance_counts.read()[i] as u32;
        triangle_mesh_data.set_changed(true);
    }
}

pub fn phosphor_update_beam_line_draw_count_system(world: &mut World) {
    let mut query = world
        .query::<&antigen_wgpu::BufferLengthComponent>()
        .with::<LineInstances>();
    let (_, line_instance_count) = query.into_iter().next().unwrap();

    let mut query = world
        .query::<&mut RenderPassDrawComponent>()
        .with::<BeamLines>();
    let (_, render_pass_draw) = query.into_iter().next().unwrap();

    render_pass_draw.1 = 0..(line_instance_count.load(Ordering::Relaxed) as u32);
}

pub fn assemble_triangle_mesh_instances_system(world: &mut World) {
    let instances = world
        .query_mut::<(
            &mut TriangleMeshInstanceComponent,
            Option<&PositionComponent>,
            Option<&RotationComponent>,
            Option<&ScaleComponent>,
        )>()
        .into_iter()
        .flat_map(
            |(entity, (triangle_mesh_instance, position, rotation, scale))| {
                let position = if let Some(position) = position {
                    **position
                } else {
                    nalgebra::Vector3::zeros()
                };

                let rotation = if let Some(rotation) = rotation {
                    **rotation
                } else {
                    nalgebra::UnitQuaternion::identity()
                };

                let scale = if let Some(scale) = scale {
                    **scale
                } else {
                    nalgebra::vector![1.0, 1.0, 1.0]
                };

                if let LazyComponent::Pending(mesh) = &**triangle_mesh_instance {
                    Some((entity, mesh.clone(), position, rotation, scale))
                } else {
                    None
                }
            },
        )
        .collect::<Vec<_>>();

    for (entity, mesh, position, rotation, scale) in instances {
        if let Some(mut builder) = triangle_mesh_instance_builder(
            world,
            &mesh,
            position.into(),
            rotation.into(),
            scale.into(),
        ) {
            world
                .get_mut::<TriangleMeshInstanceComponent>(entity)
                .unwrap()
                .set_ready();

            let copy_to_entity = vec![world.spawn(builder.build())];

            world
                .insert(
                    entity,
                    (
                        CopyToComponent::<TriangleMeshInstance, PositionComponent>::construct(
                            copy_to_entity.clone(),
                        ),
                        CopyToComponent::<TriangleMeshInstance, RotationComponent>::construct(
                            copy_to_entity.clone(),
                        ),
                        CopyToComponent::<TriangleMeshInstance, ScaleComponent>::construct(
                            copy_to_entity,
                        ),
                    ),
                )
                .unwrap();
        }
    }
}

pub fn assemble_line_mesh_instances_system(world: &mut World) {
    let instances = world
        .query_mut::<(
            &mut LineMeshInstanceComponent,
            Option<&PositionComponent>,
            Option<&RotationComponent>,
            Option<&ScaleComponent>,
        )>()
        .into_iter()
        .flat_map(
            |(entity, (line_mesh_instance, position, rotation, scale))| {
                let position = if let Some(position) = position {
                    **position
                } else {
                    nalgebra::Vector3::zeros()
                };

                let rotation = if let Some(rotation) = rotation {
                    **rotation
                } else {
                    nalgebra::UnitQuaternion::identity()
                };

                let scale = if let Some(scale) = scale {
                    **scale
                } else {
                    nalgebra::vector![1.0, 1.0, 1.0]
                };

                if let LazyComponent::Pending(mesh) = &**line_mesh_instance {
                    Some((entity, mesh.clone(), position, rotation, scale))
                } else {
                    None
                }
            },
        )
        .collect::<Vec<_>>();

    for (entity, mesh, position, rotation, scale) in instances {
        if let Some(mut builder) =
            line_mesh_instance_builder(world, position.into(), rotation.into(), scale.into(), &mesh)
        {
            world
                .get_mut::<LineMeshInstanceComponent>(entity)
                .unwrap()
                .set_ready();

            let copy_to_entity = vec![world.spawn(builder.build())];

            world
                .insert(
                    entity,
                    (
                        CopyToComponent::<LineMeshInstance, PositionComponent>::construct(
                            copy_to_entity.clone(),
                        ),
                        CopyToComponent::<LineMeshInstance, RotationComponent>::construct(
                            copy_to_entity.clone(),
                        ),
                        CopyToComponent::<LineMeshInstance, ScaleComponent>::construct(
                            copy_to_entity,
                        ),
                    ),
                )
                .unwrap();
        }
    }
}

pub fn movers_position_system(world: &mut World) {
    for (_, (position, position_offset, speed, mover_open)) in world
        .query_mut::<(
            &mut PositionComponent,
            &mut PositionOffsetComponent,
            &SpeedComponent,
            &MoverOpenComponent,
        )>()
        .into_iter()
    {
        let (offset_from, offset_to) = &mut **position_offset;

        let (from, to) = if **mover_open {
            (offset_from, offset_to)
        } else {
            (offset_to, offset_from)
        };

        let from_mag = from.magnitude();
        if from_mag > 0.0 {
            let amount = from.normalize() * from_mag.min(**speed);
            *from -= amount;
            *to += amount;
            **position += amount;
        }
    }
}

pub fn movers_rotation_system(world: &mut World) {
    for (_, (rotation, rotation_offset, speed, mover_open)) in world
        .query_mut::<(
            &mut RotationComponent,
            &mut RotationOffsetComponent,
            &SpeedComponent,
            &MoverOpenComponent,
        )>()
        .into_iter()
    {
        let (offset_from, offset_to) = &mut **rotation_offset;

        let (from, to) = if **mover_open {
            (offset_from, offset_to)
        } else {
            (offset_to, offset_from)
        };

        let from_mag = from.magnitude();
        if from_mag > 0.0 {
            let amount = from.normalize() * from_mag.min(**speed);
            *from -= amount;
            *to += amount;
            **rotation *= nalgebra::UnitQuaternion::from_euler_angles(amount.x, amount.y, amount.z);
        }
    }
}

pub fn intersection_event_output_system(world: &mut World) {
    let mut query = world.query::<&antigen_rapier3d::EventCollector>();
    for (_, event_collector) in query.into_iter() {
        for intersection in event_collector.intersection_events().iter() {
            // Find the entity corresponding to this collider
            let mut query =
                world.query::<(&ColliderComponent, &mut ColliderEventOutputComponent)>();

            for (_, (_, output)) in query
                .into_iter()
                .filter(|(_, (collider, _))| match collider {
                    LazyComponent::Ready(collider) => {
                        if *collider == intersection.collider1
                            || *collider == intersection.collider2
                        {
                            true
                        } else {
                            false
                        }
                    }
                    _ => false,
                })
            {
                output.push(*intersection);
            }
        }
    }
}

pub fn movers_event_input_system(world: &mut World) {
    for (_, (events, mover_open)) in world
        .query_mut::<(&mut MoverEventInputComponent, &mut MoverOpenComponent)>()
        .into_iter()
    {
        for event in events.drain(..) {
            match event {
                MoverEvent::Open => **mover_open = true,
                MoverEvent::Close => **mover_open = false,
            }
        }
    }
}

pub fn event_dispatch_system<T>(world: &mut World)
where
    T: std::fmt::Debug + Clone + Send + Sync + 'static,
{
    let named_entities = get_named_entities_component(world).unwrap();

    let mut query = world.query::<(&EventTargetComponent<T>, &mut EventOutputComponent<T>)>();
    for (_, (event_target, event_output)) in query.into_iter() {
        let targets = named_entities
            .get(&**event_target)
            .unwrap_or_else(|| panic!("No event target with name {}", **event_target));

        for target in targets {
            let mut query = world
                .query_one::<&mut EventInputComponent<T>>(*target)
                .unwrap();
            let event_input = query.get().unwrap();

            event_input.extend(event_output.iter().cloned());
        }
    }
}

pub fn event_transform_system<I, O, F>(world: &mut World, mut f: F)
where
    I: Send + Sync + 'static,
    O: Send + Sync + 'static,
    F: FnMut(I) -> O,
{
    for (_, (input, output)) in world
        .query_mut::<(&mut EventInputComponent<I>, &mut EventOutputComponent<O>)>()
        .with::<EventTransformComponent<I, O>>()
        .into_iter()
    {
        for event in input.drain(..) {
            output.push(f(event))
        }
    }
}

pub fn clear_event_input_system<T>(world: &mut World)
where
    T: Send + Sync + 'static,
{
    for (_, input) in world.query_mut::<&mut EventInputComponent<T>>().into_iter() {
        input.clear()
    }
}

pub fn clear_event_output_system<T>(world: &mut World)
where
    T: Send + Sync + 'static,
{
    for (_, output) in world
        .query_mut::<&mut EventOutputComponent<T>>()
        .into_iter()
    {
        output.clear()
    }
}
