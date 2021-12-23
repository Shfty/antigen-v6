use antigen_wgpu::{BindGroupComponent, BindGroupLayoutComponent, DeviceComponent, RenderAttachmentTextureView, RenderPipelineComponent, ShaderModuleComponent, SurfaceConfigurationComponent, wgpu::{
        Color, CommandEncoder, FragmentState, LoadOp, MultisampleState, Operations,
        PipelineLayoutDescriptor, PrimitiveState, RenderPassColorAttachment, RenderPassDescriptor,
        RenderPipelineDescriptor, VertexState,
    }};

use crate::demos::phosphor::BufferFlipFlopComponent;

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

        tonemap_pipeline.set_ready(pipeline);
    }

    Some(())
}

pub fn phosphor_render_tonemap(
    encoder: &mut CommandEncoder,
    render_attachment_view: &RenderAttachmentTextureView,
    tonemap_pipeline: &RenderPipelineComponent,
    buffer_flip_flop: &BufferFlipFlopComponent,
    front_bind_group: &BindGroupComponent,
    back_bind_group: &BindGroupComponent,
) -> Option<()> {
    let render_attachment_view = render_attachment_view.get()?;
    let tonemap_pipeline = tonemap_pipeline.get()?;
    let back_bind_group = back_bind_group.get()?;
    let front_bind_group = front_bind_group.get()?;
    
    println!("Phosphor render tonemap");

    // Tonemap phosphor buffer to surface
    let mut rpass = encoder.begin_render_pass(&RenderPassDescriptor {
        label: None,
        color_attachments: &[RenderPassColorAttachment {
            view: render_attachment_view,
            resolve_target: None,
            ops: Operations {
                load: LoadOp::Clear(Color::BLACK),
                store: true,
            },
        }],
        depth_stencil_attachment: None,
    });
    rpass.set_pipeline(tonemap_pipeline);
    rpass.set_bind_group(
        0,
        if **buffer_flip_flop {
            back_bind_group
        } else {
            front_bind_group
        },
        &[],
    );
    rpass.draw(0..4, 0..1);

    Some(())
}
