use antigen_wgpu::{
    buffer_size_of,
    wgpu::{
        BlendComponent, BlendFactor, BlendOperation, BlendState, ColorTargetState, ColorWrites,
        CompareFunction, DepthBiasState, DepthStencilState, Face, FragmentState, FrontFace,
        MultisampleState, PipelineLayoutDescriptor, PrimitiveState, PrimitiveTopology,
        RenderPipelineDescriptor, StencilState, TextureFormat, VertexAttribute, VertexBufferLayout,
        VertexFormat, VertexState, VertexStepMode,
    },
    BindGroupLayoutComponent, DeviceComponent, RenderPipelineComponent, ShaderModuleComponent,
};

use crate::demos::phosphor::{
    LineVertexData, MeshVertexData, HDR_TEXTURE_FORMAT,
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
    compute_bind_group_layout: &BindGroupLayoutComponent,
    beam_line_shader: &ShaderModuleComponent,
    beam_line_pipeline: &mut RenderPipelineComponent,
) -> Option<()> {
    let uniform_bind_group_layout = uniform_bind_group_layout.get()?;
    let compute_bind_group_layout = compute_bind_group_layout.get()?;
    let beam_line_shader = beam_line_shader.get()?;

    if beam_line_pipeline.is_pending() {
        let pipeline_layout = device.create_pipeline_layout(&mut PipelineLayoutDescriptor {
            label: None,
            bind_group_layouts: &[&uniform_bind_group_layout, &compute_bind_group_layout],
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
