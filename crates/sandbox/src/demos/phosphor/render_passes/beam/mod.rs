use antigen_wgpu::{
    buffer_size_of,
    wgpu::{
        BlendComponent, BlendFactor, BlendOperation, BlendState, ColorTargetState, ColorWrites,
        CommandEncoder, CompareFunction, DepthBiasState, DepthStencilState, Face, FragmentState,
        FrontFace, IndexFormat, LoadOp, MultisampleState, Operations, PipelineLayoutDescriptor,
        PrimitiveState, PrimitiveTopology, RenderPassColorAttachment,
        RenderPassDepthStencilAttachment, RenderPassDescriptor, RenderPipelineDescriptor,
        StencilState, TextureFormat, VertexAttribute, VertexBufferLayout, VertexFormat,
        VertexState, VertexStepMode,
    },
    BindGroupComponent, BindGroupLayoutComponent, BufferComponent, DeviceComponent,
    RenderPipelineComponent, ShaderModuleComponent,
};

use crate::demos::phosphor::{
    BeamBufferViewComponent, BeamDepthBufferViewComponent, BeamMultisampleViewComponent,
    LineInstanceData, LineVertexData, MeshVertexData, CLEAR_COLOR, HDR_TEXTURE_FORMAT,
};

pub fn phosphor_prepare_beam_mesh(
    device: &DeviceComponent,
    uniform_bind_group_layout: &BindGroupLayoutComponent,
    beam_mesh_shader: &ShaderModuleComponent,
    beam_mesh_pipeline: &mut RenderPipelineComponent,
) -> Option<()> {
    let uniform_bind_group_layout = uniform_bind_group_layout.get()?;
    let beam_mesh_shader = beam_mesh_shader.get()?;

    if beam_mesh_pipeline.is_pending() {
        let pipeline_layout = device.create_pipeline_layout(&mut PipelineLayoutDescriptor {
            label: None,
            bind_group_layouts: &[&uniform_bind_group_layout],
            push_constant_ranges: &[],
        });

        let pipeline = device.create_render_pipeline(&RenderPipelineDescriptor {
            label: None,
            layout: Some(&pipeline_layout),
            vertex: VertexState {
                module: &beam_mesh_shader,
                entry_point: "vs_main",
                buffers: &[VertexBufferLayout {
                    array_stride: buffer_size_of::<MeshVertexData>(),
                    step_mode: VertexStepMode::Vertex,
                    attributes: &[
                        VertexAttribute {
                            format: VertexFormat::Float32x3,
                            offset: 0,
                            shader_location: 0,
                        },
                        VertexAttribute {
                            format: VertexFormat::Float32x3,
                            offset: buffer_size_of::<[f32; 3]>(),
                            shader_location: 1,
                        },
                        VertexAttribute {
                            format: VertexFormat::Float32x3,
                            offset: buffer_size_of::<[f32; 6]>(),
                            shader_location: 2,
                        },
                        VertexAttribute {
                            format: VertexFormat::Float32,
                            offset: buffer_size_of::<[f32; 9]>(),
                            shader_location: 3,
                        },
                        VertexAttribute {
                            format: VertexFormat::Float32,
                            offset: buffer_size_of::<[f32; 10]>(),
                            shader_location: 4,
                        },
                    ],
                }],
            },
            fragment: Some(FragmentState {
                module: &beam_mesh_shader,
                entry_point: "fs_main",
                targets: &[HDR_TEXTURE_FORMAT.into()],
            }),
            primitive: PrimitiveState {
                topology: PrimitiveTopology::TriangleList,
                front_face: FrontFace::Ccw,
                cull_mode: Some(Face::Back),
                ..Default::default()
            },
            depth_stencil: Some(DepthStencilState {
                format: TextureFormat::Depth32Float,
                depth_write_enabled: true,
                depth_compare: CompareFunction::Less,
                stencil: StencilState::default(),
                bias: DepthBiasState::default(),
            }),
            multisample: MultisampleState {
                count: 4,
                ..Default::default()
            },
            multiview: None,
        });

        beam_mesh_pipeline.set_ready(pipeline);
    }

    Some(())
}

pub fn phosphor_prepare_beam_line(
    device: &DeviceComponent,
    uniform_bind_group_layout: &BindGroupLayoutComponent,
    beam_line_shader: &ShaderModuleComponent,
    beam_line_pipeline: &mut RenderPipelineComponent,
) -> Option<()> {
    let uniform_bind_group_layout = uniform_bind_group_layout.get()?;
    let beam_line_shader = beam_line_shader.get()?;

    if beam_line_pipeline.is_pending() {
        let pipeline_layout = device.create_pipeline_layout(&mut PipelineLayoutDescriptor {
            label: None,
            bind_group_layouts: &[&uniform_bind_group_layout],
            push_constant_ranges: &[],
        });

        let pipeline = device.create_render_pipeline(&RenderPipelineDescriptor {
            label: None,
            layout: Some(&pipeline_layout),
            vertex: VertexState {
                module: &beam_line_shader,
                entry_point: "vs_main",
                buffers: &[
                    VertexBufferLayout {
                        array_stride: buffer_size_of::<LineVertexData>(),
                        step_mode: VertexStepMode::Vertex,
                        attributes: &[
                            VertexAttribute {
                                format: VertexFormat::Float32x3,
                                offset: 0,
                                shader_location: 0,
                            },
                            VertexAttribute {
                                format: VertexFormat::Float32,
                                offset: buffer_size_of::<[f32; 3]>(),
                                shader_location: 1,
                            },
                        ],
                    },
                    VertexBufferLayout {
                        array_stride: buffer_size_of::<LineInstanceData>(),
                        step_mode: VertexStepMode::Instance,
                        attributes: &[
                            VertexAttribute {
                                format: VertexFormat::Float32x3,
                                offset: 0,
                                shader_location: 2,
                            },
                            VertexAttribute {
                                format: VertexFormat::Float32x3,
                                offset: buffer_size_of::<[f32; 3]>(),
                                shader_location: 3,
                            },
                            VertexAttribute {
                                format: VertexFormat::Float32x3,
                                offset: buffer_size_of::<[f32; 6]>(),
                                shader_location: 4,
                            },
                            VertexAttribute {
                                format: VertexFormat::Float32,
                                offset: buffer_size_of::<[f32; 9]>(),
                                shader_location: 5,
                            },
                            VertexAttribute {
                                format: VertexFormat::Float32,
                                offset: buffer_size_of::<[f32; 10]>(),
                                shader_location: 6,
                            },
                            VertexAttribute {
                                format: VertexFormat::Float32x3,
                                offset: buffer_size_of::<[f32; 12]>(),
                                shader_location: 7,
                            },
                            VertexAttribute {
                                format: VertexFormat::Float32x3,
                                offset: buffer_size_of::<[f32; 15]>(),
                                shader_location: 8,
                            },
                            VertexAttribute {
                                format: VertexFormat::Float32x3,
                                offset: buffer_size_of::<[f32; 18]>(),
                                shader_location: 9,
                            },
                            VertexAttribute {
                                format: VertexFormat::Float32,
                                offset: buffer_size_of::<[f32; 21]>(),
                                shader_location: 10,
                            },
                            VertexAttribute {
                                format: VertexFormat::Float32,
                                offset: buffer_size_of::<[f32; 22]>(),
                                shader_location: 11,
                            },
                        ],
                    },
                ],
            },
            fragment: Some(FragmentState {
                module: &beam_line_shader,
                entry_point: "fs_main",
                targets: &[ColorTargetState {
                    format: HDR_TEXTURE_FORMAT,
                    blend: Some(BlendState {
                        color: BlendComponent {
                            src_factor: BlendFactor::One,
                            dst_factor: BlendFactor::One,
                            operation: BlendOperation::Add,
                        },
                        alpha: BlendComponent::REPLACE,
                    }),
                    write_mask: ColorWrites::ALL,
                }],
            }),
            primitive: PrimitiveState {
                topology: PrimitiveTopology::TriangleStrip,
                front_face: FrontFace::Ccw,
                cull_mode: Some(Face::Back),
                ..Default::default()
            },
            depth_stencil: Some(DepthStencilState {
                format: TextureFormat::Depth32Float,
                depth_write_enabled: false,
                depth_compare: CompareFunction::Less,
                stencil: StencilState::default(),
                bias: DepthBiasState::default(),
            }),
            multisample: MultisampleState {
                count: 4,
                ..Default::default()
            },
            multiview: None,
        });

        beam_line_pipeline.set_ready(pipeline);
    }

    Some(())
}

pub fn phosphor_render_beam_meshes(
    encoder: &mut CommandEncoder,
    beam_multisample_view: &BeamMultisampleViewComponent,
    beam_buffer_view: &BeamBufferViewComponent,
    beam_depth_view: &BeamDepthBufferViewComponent,
    beam_mesh_pipeline: &RenderPipelineComponent,
    mesh_vertex_buffer: &BufferComponent,
    mesh_index_buffer: &BufferComponent,
    uniform_bind_group: &BindGroupComponent,
    mesh_index_count: u32,
) -> Option<()> {
    let beam_multisample_view = beam_multisample_view.get()?;
    let beam_buffer_view = beam_buffer_view.get()?;
    let beam_depth_view = beam_depth_view.get()?;
    let beam_mesh_pipeline = beam_mesh_pipeline.get()?;
    let mesh_vertex_buffer = mesh_vertex_buffer.get()?;
    let mesh_index_buffer = mesh_index_buffer.get()?;
    let uniform_bind_group = uniform_bind_group.get()?;

    println!("Phosphor render beam meshes");

    // Draw beam meshes
    println!(
        "Drawing {} mesh indices ({} triangles)",
        mesh_index_count,
        mesh_index_count / 3
    );
    let mut rpass = encoder.begin_render_pass(&RenderPassDescriptor {
        label: None,
        color_attachments: &[RenderPassColorAttachment {
            view: beam_multisample_view,
            resolve_target: Some(beam_buffer_view),
            ops: Operations {
                load: LoadOp::Clear(CLEAR_COLOR),
                store: true,
            },
        }],
        depth_stencil_attachment: Some(RenderPassDepthStencilAttachment {
            view: beam_depth_view,
            depth_ops: Some(Operations {
                load: LoadOp::Clear(1.0),
                store: true,
            }),
            stencil_ops: None,
        }),
    });
    rpass.set_pipeline(beam_mesh_pipeline);
    rpass.set_vertex_buffer(0, mesh_vertex_buffer.slice(..));
    rpass.set_index_buffer(mesh_index_buffer.slice(..), IndexFormat::Uint16);
    rpass.set_bind_group(0, uniform_bind_group, &[]);
    rpass.draw_indexed(0..mesh_index_count as u32, 0, 0..1);

    Some(())
}

pub fn phosphor_render_beam_lines(
    encoder: &mut CommandEncoder,
    beam_multisample_view: &BeamMultisampleViewComponent,
    beam_buffer_view: &BeamBufferViewComponent,
    beam_depth_view: &BeamDepthBufferViewComponent,
    beam_line_pipeline: &RenderPipelineComponent,
    line_vertex_buffer: &BufferComponent,
    line_instance_buffer: &BufferComponent,
    uniform_bind_group: &BindGroupComponent,
    line_count: u32,
) -> Option<()> {
    let beam_multisample_view = beam_multisample_view.get()?;
    let beam_buffer_view = beam_buffer_view.get()?;
    let beam_depth_view = beam_depth_view.get()?;
    let beam_line_pipeline = beam_line_pipeline.get()?;
    let line_vertex_buffer = line_vertex_buffer.get()?;
    let line_instance_buffer = line_instance_buffer.get()?;
    let uniform_bind_group = uniform_bind_group.get()?;

    println!("Phosphor render beam lines");

    // Draw beam lines
    println!("Drawing {} line instances", line_count,);
    let mut rpass = encoder.begin_render_pass(&RenderPassDescriptor {
        label: None,
        color_attachments: &[RenderPassColorAttachment {
            view: beam_multisample_view,
            resolve_target: Some(beam_buffer_view),
            ops: Operations {
                load: LoadOp::Load,
                store: true,
            },
        }],
        depth_stencil_attachment: Some(RenderPassDepthStencilAttachment {
            view: beam_depth_view,
            depth_ops: Some(Operations {
                load: LoadOp::Load,
                store: false,
            }),
            stencil_ops: None,
        }),
    });
    rpass.set_pipeline(beam_line_pipeline);
    rpass.set_vertex_buffer(0, line_vertex_buffer.slice(..));
    rpass.set_vertex_buffer(1, line_instance_buffer.slice(..));
    rpass.set_bind_group(0, uniform_bind_group, &[]);

    rpass.draw(0..14, 0..line_count as u32);

    Some(())
}
