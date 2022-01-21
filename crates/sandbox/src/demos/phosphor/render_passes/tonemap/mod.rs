use antigen_wgpu::{
    wgpu::{
        FragmentState, MultisampleState, PipelineLayoutDescriptor, PrimitiveState,
        RenderPipelineDescriptor, VertexState,
    },
    BindGroupLayoutComponent, DeviceComponent, RenderPipelineComponent, ShaderModuleComponent,
    SurfaceConfigurationComponent,
};

pub fn phosphor_prepare_tonemap(
    device: &DeviceComponent,
    phosphor_bind_group_layout: &BindGroupLayoutComponent,
    tonemap_shader: &ShaderModuleComponent,
    surface_config: &SurfaceConfigurationComponent,
    tonemap_pipeline: &mut RenderPipelineComponent,
) -> Option<()> {
    let tonemap_shader = tonemap_shader.get()?;
    let phosphor_bind_group_layout = phosphor_bind_group_layout.get()?;

    if tonemap_pipeline.is_pending() {
        // Tonemap pipeline
        let pipeline_layout = device.create_pipeline_layout(&mut PipelineLayoutDescriptor {
            label: None,
            bind_group_layouts: &[&phosphor_bind_group_layout],
            push_constant_ranges: &[],
        });

        println!("Creating tonemap pipeline");
        let pipeline = device.create_render_pipeline(&RenderPipelineDescriptor {
            label: None,
            layout: Some(&pipeline_layout),
            vertex: VertexState {
                module: &tonemap_shader,
                entry_point: "vs_main",
                buffers: &[],
            },
            fragment: Some(FragmentState {
                module: &tonemap_shader,
                entry_point: "fs_main",
                targets: &[surface_config.format.into()],
            }),
            primitive: PrimitiveState::default(),
            depth_stencil: None,
            multisample: MultisampleState::default(),
            multiview: None,
        });

        tonemap_pipeline.set_ready_with(pipeline);
    }

    Some(())
}
