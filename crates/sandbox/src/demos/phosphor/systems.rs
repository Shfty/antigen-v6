use std::time::Instant;

use super::*;
use antigen_core::{Changed, ChangedTrait, Indirect};

use antigen_wgpu::{
    wgpu::{
        BindGroupDescriptor, BindGroupEntry, BindGroupLayoutDescriptor, BindGroupLayoutEntry,
        BindingType, BufferBindingType, BufferSize, ComputePipelineDescriptor, Extent3d,
        PipelineLayoutDescriptor, ShaderStages,
    },
    BindGroupComponent, BindGroupLayoutComponent, BufferComponent, DeviceComponent,
    RenderPassDrawComponent, RenderPassDrawIndexedComponent, SamplerComponent,
    SurfaceConfigurationComponent, TextureDescriptorComponent, TextureViewComponent,
    TextureViewDescriptorComponent,
};

use antigen_winit::{winit::event::WindowEvent, WindowComponent, WindowEventComponent};
use hecs::World;

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
                    min_binding_size: BufferSize::new(144),
                },
                count: None,
            }],
        });

        uniform_bind_group_layout.set_ready(bind_group_layout);
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

        uniform_bind_group.set_ready(bind_group);
    }

    Some(())
}

pub fn phosphor_prepare_compute(
    device: &DeviceComponent,
    mesh_vertex_buffer: &BufferComponent,
    line_index_buffer: &BufferComponent,
    bind_group_layout: &mut BindGroupLayoutComponent,
    bind_group: &mut BindGroupComponent,
) -> Option<()> {
    let mesh_vertex_buffer = mesh_vertex_buffer.get()?;
    let line_index_buffer = line_index_buffer.get()?;

    let bind_group_layout = match bind_group_layout.get() {
        Some(bind_group_layout) => bind_group_layout,
        None => {
            let compute_bind_group_layout =
                device.create_bind_group_layout(&BindGroupLayoutDescriptor {
                    label: Some("Compute Bind Group Layout"),
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
                                has_dynamic_offset: false,
                                min_binding_size: BufferSize::new(4),
                            },
                            count: None,
                        },
                    ],
                });

            bind_group_layout.set_ready(compute_bind_group_layout);
            bind_group_layout.get().unwrap()
        }
    };

    if bind_group.is_pending() {
        let compute_bind_group = device.create_bind_group(&BindGroupDescriptor {
            layout: &bind_group_layout,
            entries: &[
                BindGroupEntry {
                    binding: 0,
                    resource: mesh_vertex_buffer.as_entire_binding(),
                },
                BindGroupEntry {
                    binding: 1,
                    resource: line_index_buffer.as_entire_binding(),
                },
            ],
            label: None,
        });

        bind_group.set_ready(compute_bind_group);
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

    let mut query = world.query::<(&BufferComponent,)>().with::<MeshVertex>();
    let (_, (mesh_vertex_buffer,)) = query.into_iter().next()?;

    let mut query = world.query::<(&BufferComponent,)>().with::<LineIndex>();
    let (_, (line_index_buffer,)) = query.into_iter().next()?;

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
        .query::<(
            &mut BindGroupLayoutComponent,
            &mut BindGroupComponent,
        )>()
        .with::<ComputeLineInstances>();
    let (_, (compute_bind_group_layout, compute_bind_group)) =
        query.into_iter().next()?;
    println!("Fetched compute pass entity");

    phosphor_prepare_compute(
        device,
        mesh_vertex_buffer,
        line_index_buffer,
        compute_bind_group_layout,
        compute_bind_group,
    )?;

    let mut query = world
        .query::<(&ShaderModuleComponent, &mut RenderPipelineComponent)>()
        .with::<BeamMesh>();

    let (_, (beam_mesh_shader, beam_mesh_pipeline)) = query.into_iter().next()?;
    println!("Fetched beam mesh pass entity");

    phosphor_prepare_beam_mesh(
        device,
        uniform_bind_group_layout,
        beam_mesh_shader,
        beam_mesh_pipeline,
    )?;

    let mut query = world
        .query::<(&ShaderModuleComponent, &mut RenderPipelineComponent)>()
        .with::<BeamLine>();
    let (_, (beam_line_shader, beam_line_pipeline)) = query.into_iter().next()?;
    println!("Fetched beam line pass entity");

    phosphor_prepare_beam_line(
        device,
        uniform_bind_group_layout,
        compute_bind_group_layout,
        beam_line_shader,
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

    for (entity, (origin, oscilloscope, vertex_data)) in world
        .query::<(
            &OriginComponent,
            &Oscilloscope,
            &mut Changed<MeshVertexDataComponent>,
        )>()
        .into_iter()
    {
        println!("Updating oscilloscope for entity {:?}", entity);

        {
            let (x, y, z) = **origin;
            let (fx, fy, fz) = oscilloscope.eval(***total_time);

            vertex_data[0] = vertex_data[1];
            vertex_data[0].intensity += vertex_data[0].delta_intensity * ***delta_time;

            vertex_data[1].position[0] = x + fx;
            vertex_data[1].position[1] = y + fy;
            vertex_data[1].position[2] = z + fz;
        }

        vertex_data.set_changed(true);
    }
}

pub fn phosphor_resize_system(world: &mut World) {
    let mut query = world.query::<&PhosphorRenderer>();
    for (entity, _) in query.into_iter() {
        phosphor_resize(world, entity);
    }
}

pub fn phosphor_resize(world: &World, entity: Entity) {
    let mut query = world
        .query_one::<&Indirect<&SurfaceConfigurationComponent>>(entity)
        .unwrap();
    let mut query = query.get().unwrap().get(world);
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
        .with::<Perspective>();
    let (_, (perspective_matrix,)) = query.into_iter().next().unwrap();

    let mut query = world
        .query::<(&mut Changed<OrthographicMatrixComponent>,)>()
        .with::<Orthographic>();
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

    ***perspective_matrix = super::perspective_matrix(aspect, (0.0, 0.0), 1.0, 500.0);
    perspective_matrix.set_changed(true);

    ***orthographic_matrix = super::orthographic_matrix(aspect, 200.0, 1.0, 500.0);
    orthographic_matrix.set_changed(true);
}

pub fn phosphor_cursor_moved_system(world: &mut World) {
    for (_, (_, window, surface_config)) in world
        .query::<(
            &PhosphorRenderer,
            &Indirect<&WindowComponent>,
            &Indirect<&SurfaceConfigurationComponent>,
        )>()
        .into_iter()
    {
        let mut query = world
            .query::<(&mut Changed<PerspectiveMatrixComponent>,)>()
            .with::<Perspective>();
        let (_, (perspective_matrix,)) = query.into_iter().next().unwrap();

        let mut query = window.get(world);
        let window = query.get().expect("No indirect WindowComponent");
        let window = if let Some(window) = window.get() {
            window
        } else {
            continue;
        };

        let mut query = surface_config.get(world);
        let surface_config = query
            .get()
            .expect("No indirect SurfaceConfigurationComponent");

        let mut query = world.query::<&WindowEventComponent>();
        let (_, window_event) = query.into_iter().next().expect("No WindowEventComponent");

        let (window_id, position) =
            if let (Some(window_id), Some(WindowEvent::CursorMoved { position, .. })) =
                &*window_event
            {
                (window_id, position)
            } else {
                continue;
            };

        if window.id() != *window_id {
            continue;
        }

        let norm_x = ((position.x as f32 / surface_config.width as f32) * 2.0) - 1.0;
        let norm_y = ((position.y as f32 / surface_config.height as f32) * 2.0) - 1.0;

        ***perspective_matrix = super::perspective_matrix(
            surface_config.width as f32 / surface_config.height as f32,
            (-norm_x, norm_y),
            1.0,
            500.0,
        );
        perspective_matrix.set_changed(true);
    }
}

pub fn phosphor_update_compute_indirect(world: &mut World) {
    let mut query = world
        .query::<&antigen_wgpu::BufferLengthComponent>()
        .with::<LineIndex>();
    let (_, line_index_count) = query.into_iter().next().unwrap();

    let mut query = world.query::<&mut Changed<[u32; 3]>>();
    let (_, compute_indirect) = query.into_iter().next().unwrap();

    let line_count = **line_index_count / 2;
    if (**compute_indirect)[0] != line_count as u32 {
        compute_indirect[0] = line_count as u32;
        compute_indirect.set_changed(true);
    }
}

pub fn phosphor_update_beam_mesh_draw_count_system(world: &mut World) {
    let mut query = world
        .query::<&antigen_wgpu::BufferLengthComponent>()
        .with::<MeshIndex>();
    let (_, mesh_index_count) = query.into_iter().next().unwrap();

    let mut query = world
        .query::<&mut RenderPassDrawIndexedComponent>()
        .with::<BeamMesh>();
    let (_, render_pass_draw_indexed) = query.into_iter().next().unwrap();

    render_pass_draw_indexed.0 = 0..(**mesh_index_count as u32);
}

pub fn phosphor_update_beam_line_draw_count_system(world: &mut World) {
    let mut query = world
        .query::<&antigen_wgpu::BufferLengthComponent>()
        .with::<LineIndex>();
    let (_, line_index_count) = query.into_iter().next().unwrap();

    let mut query = world
        .query::<&mut RenderPassDrawComponent>()
        .with::<BeamLine>();
    let (_, render_pass_draw) = query.into_iter().next().unwrap();

    render_pass_draw.1 = 0..((**line_index_count as u32) / 2);
}
