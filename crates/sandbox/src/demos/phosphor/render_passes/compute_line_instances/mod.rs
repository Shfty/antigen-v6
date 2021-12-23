use antigen_wgpu::{BindGroupComponent, BindGroupLayoutComponent, BufferComponent, ComputePipelineComponent, DeviceComponent, ShaderModuleComponent, wgpu::{
        BindGroupDescriptor, BindGroupEntry, BindGroupLayoutDescriptor, BindGroupLayoutEntry,
        BindingType, BufferBindingType, BufferSize, CommandEncoder, ComputePassDescriptor,
        ComputePipelineDescriptor, PipelineLayoutDescriptor, ShaderStages,
    }};

pub fn phosphor_prepare_compute(
    device: &DeviceComponent,
    shader_module: &ShaderModuleComponent,
    mesh_vertex_buffer: &BufferComponent,
    line_index_buffer: &BufferComponent,
    line_instance_buffer: &BufferComponent,
    bind_group_layout: &mut BindGroupLayoutComponent,
    bind_group: &mut BindGroupComponent,
    pipeline: &mut ComputePipelineComponent,
) -> Option<()> {
    let shader_module = shader_module.get()?;
    let mesh_vertex_buffer = mesh_vertex_buffer.get()?;
    let line_index_buffer = line_index_buffer.get()?;
    let line_instance_buffer = line_instance_buffer.get()?;

    let bind_group_layout = match bind_group_layout.get() {
        Some(bind_group_layout) => bind_group_layout,
        None => {
            let compute_bind_group_layout =
                device.create_bind_group_layout(&BindGroupLayoutDescriptor {
                    label: Some("Compute Bind Group Layout"),
                    entries: &[
                        BindGroupLayoutEntry {
                            binding: 0,
                            visibility: ShaderStages::COMPUTE,
                            ty: BindingType::Buffer {
                                ty: BufferBindingType::Storage { read_only: true },
                                has_dynamic_offset: false,
                                min_binding_size: BufferSize::new(48),
                            },
                            count: None,
                        },
                        BindGroupLayoutEntry {
                            binding: 1,
                            visibility: ShaderStages::COMPUTE,
                            ty: BindingType::Buffer {
                                ty: BufferBindingType::Storage { read_only: true },
                                has_dynamic_offset: false,
                                min_binding_size: BufferSize::new(4),
                            },
                            count: None,
                        },
                        BindGroupLayoutEntry {
                            binding: 2,
                            visibility: ShaderStages::COMPUTE,
                            ty: BindingType::Buffer {
                                ty: BufferBindingType::Storage { read_only: false },
                                has_dynamic_offset: false,
                                min_binding_size: BufferSize::new(96),
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
                BindGroupEntry {
                    binding: 2,
                    resource: line_instance_buffer.as_entire_binding(),
                },
            ],
            label: None,
        });

        bind_group.set_ready(compute_bind_group);
    }

    // Compute bind group and pipeline
    if pipeline.is_pending() {
        // Compute pipeline
        let compute_pipeline_layout =
            device.create_pipeline_layout(&mut PipelineLayoutDescriptor {
                label: None,
                bind_group_layouts: &[&bind_group_layout],
                push_constant_ranges: &[],
            });

        let compute_pipeline = device.create_compute_pipeline(&ComputePipelineDescriptor {
            label: Some("Compute Pipeline"),
            layout: Some(&compute_pipeline_layout),
            module: shader_module,
            entry_point: "main",
        });

        pipeline.set_ready(compute_pipeline);
    }

    Some(())
}

pub fn phosphor_render_compute(
    encoder: &mut CommandEncoder,
    compute_pipeline: &ComputePipelineComponent,
    compute_bind_group: &BindGroupComponent,
    line_count: u32,
) -> Option<()> {
    let compute_pipeline = compute_pipeline.get()?;
    let compute_bind_group = compute_bind_group.get()?;

    println!("Phosphor render compute");

    // Compute line instances
    let mut cpass = encoder.begin_compute_pass(&ComputePassDescriptor {
        label: Some("Compute Pass"),
    });
    cpass.set_pipeline(compute_pipeline);
    cpass.set_bind_group(0, compute_bind_group, &[]);
    cpass.dispatch(line_count as u32, 1, 1);

    Some(())
}
