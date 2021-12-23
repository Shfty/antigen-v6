use std::time::Instant;

use super::*;
use antigen_core::{Changed, ChangedTrait, Indirect, Usage};

use antigen_wgpu::{
    wgpu::{
        BindGroupDescriptor, BindGroupEntry, BindGroupLayoutDescriptor, BindGroupLayoutEntry,
        BindingType, BufferBindingType, BufferSize, CommandEncoderDescriptor, Extent3d,
        ShaderStages,
    },
    BindGroupComponent, BindGroupLayoutComponent, CommandBuffersComponent, DeviceComponent,
    RenderAttachmentTextureView, SurfaceConfigurationComponent, TextureDescriptorComponent,
    TextureViewDescriptorComponent,
};

use antigen_winit::{winit::event::WindowEvent, WindowComponent, WindowEventComponent};
use hecs::World;

#[derive(hecs::Query)]
struct TextureViews<'a> {
    beam_buffer: &'a BeamBufferViewComponent,
    beam_depth: &'a BeamDepthBufferViewComponent,
    beam_multisample: &'a BeamMultisampleViewComponent,
    phosphor_front: &'a PhosphorFrontBufferViewComponent,
    phosphor_back: &'a PhosphorBackBufferViewComponent,
}

#[derive(hecs::Query)]
struct BeamPhosphorTextureViews<'a> {
    beam_buffer: &'a BeamBufferViewComponent,
    phosphor_front: &'a PhosphorFrontBufferViewComponent,
    phosphor_back: &'a PhosphorBackBufferViewComponent,
}

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
    uniform_buffer: &UniformBufferComponent,
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

pub fn phosphor_prepare(world: &World, entity: Entity, device: &DeviceComponent) -> Option<()> {
    let mut query = world.query_one::<BeamPhosphorTextureViews>(entity).unwrap();
    let texture_views = query.get().unwrap();

    let mut query = world.query_one::<&LinearSamplerComponent>(entity).unwrap();
    let sampler = query.get().unwrap();

    let mut query = world
        .query_one::<&Indirect<SurfaceConfigurationComponent>>(entity)
        .unwrap();
    let mut query = query.get().unwrap().get(world);
    let surface_config = query.get().unwrap();

    let mut query = world
        .query::<(
            &UniformBufferComponent,
            &mut BindGroupLayoutComponent,
            &mut BindGroupComponent,
        )>()
        .with::<Uniform>();
    let (_, (uniform_buffer, uniform_bind_group_layout, uniform_bind_group)) =
        query.into_iter().next()?;

    let mut query = world
        .query::<(&MeshVertexBufferComponent,)>()
        .with::<MeshVertex>();
    let (_, (mesh_vertex_buffer,)) = query.into_iter().next()?;

    let mut query = world
        .query::<(&LineIndexBufferComponent,)>()
        .with::<LineIndex>();
    let (_, (line_index_buffer,)) = query.into_iter().next()?;

    let mut query = world
        .query::<(&LineInstanceBufferComponent,)>()
        .with::<LineInstance>();
    let (_, (line_instance_buffer,)) = query.into_iter().next()?;

    phosphor_prepare_uniform_bind_group(
        device,
        uniform_buffer,
        uniform_bind_group_layout,
        uniform_bind_group,
    );

    let mut query = world
        .query::<(
            &ShaderModuleComponent,
            &mut ComputePipelineComponent,
            &mut BindGroupLayoutComponent,
            &mut BindGroupComponent,
        )>()
        .with::<ComputeLineInstances>();
    let (_, (compute_shader, compute_pipeline, compute_bind_group_layout, compute_bind_group)) =
        query.into_iter().next()?;
    println!("Fetched compute pass entity");

    phosphor_prepare_compute(
        device,
        compute_shader,
        mesh_vertex_buffer,
        line_index_buffer,
        line_instance_buffer,
        compute_bind_group_layout,
        compute_bind_group,
        compute_pipeline,
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
        texture_views.beam_buffer,
        texture_views.phosphor_front,
        texture_views.phosphor_back,
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

#[derive(hecs::Query)]
struct BufferDescriptorMutQuery<'a> {
    beam: &'a mut Usage<BeamBuffer, TextureDescriptorComponent<'static>>,
    beam_depth: &'a mut Usage<BeamDepthBuffer, TextureDescriptorComponent<'static>>,
    beam_multisample: &'a mut Usage<BeamMultisample, TextureDescriptorComponent<'static>>,
    phosphor_front: &'a mut Usage<PhosphorFrontBuffer, TextureDescriptorComponent<'static>>,
    phosphor_back: &'a mut Usage<PhosphorBackBuffer, TextureDescriptorComponent<'static>>,
}

#[derive(hecs::Query)]
struct BufferViewDescriptorMutQuery<'a> {
    beam: &'a mut Usage<BeamBuffer, TextureViewDescriptorComponent<'static>>,
    beam_depth: &'a mut Usage<BeamDepthBuffer, TextureViewDescriptorComponent<'static>>,
    beam_multisample: &'a mut Usage<BeamMultisample, TextureViewDescriptorComponent<'static>>,
    phosphor_front: &'a mut Usage<PhosphorFrontBuffer, TextureViewDescriptorComponent<'static>>,
    phosphor_back: &'a mut Usage<PhosphorBackBuffer, TextureViewDescriptorComponent<'static>>,
}

pub fn phosphor_resize(world: &World, entity: Entity) {
    let mut query = world
        .query_one::<&Indirect<SurfaceConfigurationComponent>>(entity)
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

    let mut query = world.query_one::<BufferDescriptorMutQuery>(entity).unwrap();
    let buffer_descs = query.get().unwrap();

    let mut query = world
        .query_one::<BufferViewDescriptorMutQuery>(entity)
        .unwrap();
    let view_descs = query.get().unwrap();

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

    buffer_descs.beam.size = extent;
    buffer_descs.beam_depth.size = extent;
    buffer_descs.beam_multisample.size = extent;
    buffer_descs.phosphor_front.size = extent;
    buffer_descs.phosphor_back.size = extent;

    buffer_descs.beam.set_changed(true);
    buffer_descs.beam_depth.set_changed(true);
    buffer_descs.beam_multisample.set_changed(true);
    buffer_descs.phosphor_front.set_changed(true);
    buffer_descs.phosphor_back.set_changed(true);

    view_descs.beam.set_changed(true);
    view_descs.beam_depth.set_changed(true);
    view_descs.beam_multisample.set_changed(true);
    view_descs.phosphor_front.set_changed(true);
    view_descs.phosphor_back.set_changed(true);

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
            &Indirect<WindowComponent>,
            &Indirect<SurfaceConfigurationComponent>,
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

pub fn phosphor_render_system(world: &mut World) {
    // Fetch resources
    let mut query = world.query::<&DeviceComponent>();
    let (_, device) = query.into_iter().next().unwrap();

    let mut query = world.query::<&PhosphorRenderer>();
    for (entity, _) in query.into_iter() {
        phosphor_render(world, entity, device);
    }
}

pub fn phosphor_render(world: &World, entity: Entity, device: &DeviceComponent) -> Option<()> {
    let mut query = world.query_one::<TextureViews>(entity).unwrap();
    let texture_views = query.get().unwrap();

    let mut query = world
        .query_one::<(
            &MeshIndexCountComponent,
            &LineIndexCountComponent,
            &mut BufferFlipFlopComponent,
            &mut CommandBuffersComponent,
            &Indirect<RenderAttachmentTextureView>,
        )>(entity)
        .unwrap();

    let (
        mesh_index_count,
        line_index_count,
        buffer_flip_flop,
        command_buffers,
        render_attachment_view,
    ) = query.get().unwrap();

    let mesh_index_count = **mesh_index_count;
    let line_index_count = **line_index_count;
    let line_count = line_index_count / 2;

    let mut query = render_attachment_view.get(world);
    let render_attachment_view = query.get().unwrap();

    let mut query = world
        .query::<(&mut BindGroupComponent,)>()
        .with::<Uniform>();
    let (_, (uniform_bind_group,)) = query.into_iter().next()?;

    let mut query = world
        .query::<(&mut MeshVertexBufferComponent,)>()
        .with::<MeshVertex>();
    let (_, (mesh_vertex_buffer,)) = query.into_iter().next()?;

    let mut query = world
        .query::<(&mut MeshIndexBufferComponent,)>()
        .with::<MeshIndex>();
    let (_, (mesh_index_buffer,)) = query.into_iter().next()?;

    let mut query = world
        .query::<(&mut LineVertexBufferComponent,)>()
        .with::<LineVertex>();
    let (_, (line_vertex_buffer,)) = query.into_iter().next()?;

    let mut query = world
        .query::<(&mut LineInstanceBufferComponent,)>()
        .with::<LineInstance>();
    let (_, (line_instance_buffer,)) = query.into_iter().next()?;

    let mut query = world
        .query::<(&mut ComputePipelineComponent, &mut BindGroupComponent)>()
        .with::<ComputeLineInstances>();
    let (_, (compute_pipeline, compute_bind_group)) = query.into_iter().next().unwrap();

    let mut query = world
        .query::<(&RenderPipelineComponent,)>()
        .with::<PhosphorDecay>();
    let (_, (phosphor_decay_pipeline,)) = query.into_iter().next().unwrap();

    let mut query = world
        .query::<(&BindGroupComponent,)>()
        .with::<PhosphorFrontBuffer>();
    let (_, (front_bind_group,)) = query.into_iter().next()?;

    let mut query = world
        .query::<(&BindGroupComponent,)>()
        .with::<PhosphorBackBuffer>();
    let (_, (back_bind_group,)) = query.into_iter().next()?;

    let mut query = world
        .query::<(&RenderPipelineComponent,)>()
        .with::<BeamMesh>();
    let (_, (beam_mesh_pipeline,)) = query.into_iter().next().unwrap();

    let mut query = world
        .query::<(&RenderPipelineComponent,)>()
        .with::<BeamLine>();
    let (_, (beam_line_pipeline,)) = query.into_iter().next().unwrap();

    let mut query = world
        .query::<(&RenderPipelineComponent,)>()
        .with::<Tonemap>();
    let (_, (tonemap_pipeline,)) = query.into_iter().next().unwrap();

    let mut encoder = device.create_command_encoder(&CommandEncoderDescriptor { label: None });

    phosphor_render_compute(
        &mut encoder,
        compute_pipeline,
        compute_bind_group,
        line_count as u32,
    );

    phosphor_render_beam_meshes(
        &mut encoder,
        texture_views.beam_multisample,
        texture_views.beam_buffer,
        texture_views.beam_depth,
        beam_mesh_pipeline,
        mesh_vertex_buffer,
        mesh_index_buffer,
        uniform_bind_group,
        mesh_index_count as u32,
    );

    phosphor_render_beam_lines(
        &mut encoder,
        texture_views.beam_multisample,
        texture_views.beam_buffer,
        texture_views.beam_depth,
        beam_line_pipeline,
        line_vertex_buffer,
        line_instance_buffer,
        uniform_bind_group,
        line_count as u32,
    );

    phosphor_render_phosphor_decay(
        &mut encoder,
        buffer_flip_flop,
        texture_views.phosphor_front,
        texture_views.phosphor_back,
        phosphor_decay_pipeline,
        uniform_bind_group,
        front_bind_group,
        back_bind_group,
    );

    phosphor_render_tonemap(
        &mut encoder,
        render_attachment_view,
        tonemap_pipeline,
        buffer_flip_flop,
        front_bind_group,
        back_bind_group,
    );

    // Finish encoding
    command_buffers.push(encoder.finish());

    // Flip buffer flag
    **buffer_flip_flop = !**buffer_flip_flop;

    Some(())
}
