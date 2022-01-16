use std::ops::Range;

use antigen_core::{Construct, Indirect, Usage};
use hecs::{Entity, EntityBuilder, World};
use wgpu::{
    BufferAddress, Color, DynamicOffset, IndexFormat, Operations, RenderPassColorAttachment,
    RenderPassDepthStencilAttachment, RenderPassDescriptor, ShaderStages,
};

use crate::{
    BindGroupComponent, BufferComponent, CommandEncoderComponent, PassOrderComponent,
    PushConstantQuery, RenderPipelineComponent, TextureViewComponent,
};

pub enum RenderPassTag {}

pub type RenderPassLabelComponent = Usage<RenderPassTag, Option<String>>;
pub type RenderPassColorAttachmentsComponent = Usage<
    RenderPassTag,
    Vec<(
        Indirect<&'static TextureViewComponent>,
        Option<Indirect<&'static TextureViewComponent>>,
        Operations<Color>,
    )>,
>;
pub type RenderPassDepthAttachmentComponent = Usage<
    RenderPassTag,
    Option<(
        Indirect<&'static TextureViewComponent>,
        Option<Operations<f32>>,
        Option<Operations<u32>>,
    )>,
>;
pub type RenderPassPipelineComponent =
    Usage<RenderPassTag, Indirect<&'static RenderPipelineComponent>>;
pub type RenderPassVertexBuffersComponent =
    Usage<RenderPassTag, Vec<(Indirect<&'static BufferComponent>, Range<BufferAddress>)>>;
pub type RenderPassIndexBufferComponent = Usage<
    RenderPassTag,
    Option<(
        Indirect<&'static BufferComponent>,
        Range<BufferAddress>,
        IndexFormat,
    )>,
>;
pub type RenderPassBindGroupsComponent =
    Usage<RenderPassTag, Vec<(Indirect<&'static BindGroupComponent>, Vec<DynamicOffset>)>>;
pub type RenderPassPushConstantsComponent =
    Usage<RenderPassTag, Vec<(Indirect<PushConstantQuery<'static>>, ShaderStages)>>;
pub type RenderPassEncoderComponent =
    Usage<RenderPassTag, Indirect<&'static mut CommandEncoderComponent>>;

pub type RenderPassViewportComponent = Usage<RenderPassTag, (f32, f32, f32, f32, f32, f32)>;
pub type RenderPassScissorRectComponent = Usage<RenderPassTag, (u32, u32, u32, u32)>;

pub type RenderPassBlendConstantComponent = Usage<RenderPassTag, Color>;
pub type RenderPassStencilReferenceComponent = Usage<RenderPassTag, u32>;

pub type RenderPassDrawComponent = Usage<RenderPassTag, (Range<u32>, Range<u32>)>;
pub type RenderPassDrawIndexedComponent = Usage<RenderPassTag, (Range<u32>, i32, Range<u32>)>;

pub enum DrawIndirect {}
pub enum DrawIndexedIndirect {}

pub type RenderPassDrawIndirectComponent =
    Usage<(RenderPassTag, DrawIndirect), (Indirect<&'static BufferComponent>, BufferAddress)>;
pub type RenderPassDrawIndexedIndirectComponent = Usage<
    (RenderPassTag, DrawIndexedIndirect),
    (Indirect<&'static BufferComponent>, BufferAddress),
>;

pub enum RenderPassBundle {}

impl RenderPassBundle {
    fn builder_impl(
        builder: &mut EntityBuilder,
        order: usize,
        label: Option<String>,
        color_attachments: Vec<(Entity, Option<Entity>, Operations<Color>)>,
        depth_attachment: Option<(Entity, Option<Operations<f32>>, Option<Operations<u32>>)>,
        pipeline: Entity,
        vertex_buffers: Vec<(Entity, Range<BufferAddress>)>,
        index_buffers: Option<(Entity, Range<BufferAddress>, IndexFormat)>,
        bind_groups: Vec<(Entity, Vec<DynamicOffset>)>,
        push_constants: Vec<(Entity, ShaderStages)>,
        blend_constant: Option<Color>,
        stencil_reference: Option<u32>,
        viewport: Option<(f32, f32, f32, f32, f32, f32)>,
        scissor_rect: Option<(u32, u32, u32, u32)>,
        encoder: Entity,
    ) {
        builder.add(PassOrderComponent::construct(order));

        builder.add(RenderPassLabelComponent::construct(label));

        let color_attachments = RenderPassColorAttachmentsComponent::construct(
            color_attachments
                .into_iter()
                .map(|(view, resolve_target, ops)| {
                    (
                        Indirect::construct(view),
                        resolve_target.map(Indirect::construct),
                        ops,
                    )
                })
                .collect(),
        );
        builder.add(color_attachments);

        let depth_attachment = RenderPassDepthAttachmentComponent::construct(depth_attachment.map(
            |(view, depth_ops, stencil_ops)| (Indirect::construct(view), depth_ops, stencil_ops),
        ));
        builder.add(depth_attachment);

        let pipeline = RenderPassPipelineComponent::construct(Indirect::construct(pipeline));
        builder.add(pipeline);

        let vertex_buffers = RenderPassVertexBuffersComponent::construct(
            vertex_buffers
                .into_iter()
                .map(|(entity, range)| (Indirect::construct(entity), range))
                .collect(),
        );
        builder.add(vertex_buffers);

        let index_buffer = RenderPassIndexBufferComponent::construct(
            index_buffers
                .map(|(entity, range, format)| (Indirect::construct(entity), range, format)),
        );
        builder.add(index_buffer);

        let bind_groups = RenderPassBindGroupsComponent::construct(
            bind_groups
                .into_iter()
                .map(|(entity, offsets)| (Indirect::construct(entity), offsets))
                .collect(),
        );
        builder.add(bind_groups);

        let push_constants = RenderPassPushConstantsComponent::construct(
            push_constants
                .into_iter()
                .map(|(entity, shader_stages)| (Indirect::construct(entity), shader_stages))
                .collect(),
        );
        builder.add(push_constants);

        if let Some(blend_constant) = blend_constant {
            builder.add(RenderPassBlendConstantComponent::construct(blend_constant));
        }

        if let Some(stencil_reference) = stencil_reference {
            builder.add(RenderPassStencilReferenceComponent::construct(
                stencil_reference,
            ));
        }

        if let Some(viewport) = viewport {
            builder.add(RenderPassViewportComponent::construct(viewport));
        }

        if let Some(scissor_rect) = scissor_rect {
            builder.add(RenderPassScissorRectComponent::construct(scissor_rect));
        }

        let encoder = RenderPassEncoderComponent::construct(encoder);
        builder.add(encoder);
    }

    pub fn draw(
        order: usize,
        label: Option<String>,
        color_attachments: Vec<(Entity, Option<Entity>, Operations<Color>)>,
        depth_attachment: Option<(Entity, Option<Operations<f32>>, Option<Operations<u32>>)>,
        pipeline: Entity,
        vertex_buffers: Vec<(Entity, Range<BufferAddress>)>,
        index_buffers: Option<(Entity, Range<BufferAddress>, IndexFormat)>,
        bind_groups: Vec<(Entity, Vec<DynamicOffset>)>,
        push_constants: Vec<(Entity, ShaderStages)>,
        blend_constant: Option<Color>,
        stencil_reference: Option<u32>,
        viewport: Option<(f32, f32, f32, f32, f32, f32)>,
        scissor_rect: Option<(u32, u32, u32, u32)>,
        draw: (Range<u32>, Range<u32>),
        encoder: Entity,
    ) -> EntityBuilder {
        let mut builder = EntityBuilder::new();

        Self::builder_impl(
            &mut builder,
            order,
            label,
            color_attachments,
            depth_attachment,
            pipeline,
            vertex_buffers,
            index_buffers,
            bind_groups,
            push_constants,
            blend_constant,
            stencil_reference,
            viewport,
            scissor_rect,
            encoder,
        );

        let draw = RenderPassDrawComponent::construct(draw);
        builder.add(draw);

        builder
    }

    pub fn draw_indexed(
        order: usize,
        label: Option<String>,
        color_attachments: Vec<(Entity, Option<Entity>, Operations<Color>)>,
        depth_attachment: Option<(Entity, Option<Operations<f32>>, Option<Operations<u32>>)>,
        pipeline: Entity,
        vertex_buffers: Vec<(Entity, Range<BufferAddress>)>,
        index_buffers: Option<(Entity, Range<BufferAddress>, IndexFormat)>,
        bind_groups: Vec<(Entity, Vec<DynamicOffset>)>,
        push_constants: Vec<(Entity, ShaderStages)>,
        blend_constant: Option<Color>,
        stencil_reference: Option<u32>,
        viewport: Option<(f32, f32, f32, f32, f32, f32)>,
        scissor_rect: Option<(u32, u32, u32, u32)>,
        draw_indexed: (Range<u32>, i32, Range<u32>),
        encoder: Entity,
    ) -> EntityBuilder {
        let mut builder = EntityBuilder::new();

        Self::builder_impl(
            &mut builder,
            order,
            label,
            color_attachments,
            depth_attachment,
            pipeline,
            vertex_buffers,
            index_buffers,
            bind_groups,
            push_constants,
            blend_constant,
            stencil_reference,
            viewport,
            scissor_rect,
            encoder,
        );

        let draw = RenderPassDrawIndexedComponent::construct(draw_indexed);
        builder.add(draw);

        builder
    }

    pub fn draw_indirect(
        order: usize,
        label: Option<String>,
        color_attachments: Vec<(Entity, Option<Entity>, Operations<Color>)>,
        depth_attachment: Option<(Entity, Option<Operations<f32>>, Option<Operations<u32>>)>,
        pipeline: Entity,
        vertex_buffers: Vec<(Entity, Range<BufferAddress>)>,
        index_buffers: Option<(Entity, Range<BufferAddress>, IndexFormat)>,
        bind_groups: Vec<(Entity, Vec<DynamicOffset>)>,
        push_constants: Vec<(Entity, ShaderStages)>,
        blend_constant: Option<Color>,
        stencil_reference: Option<u32>,
        viewport: Option<(f32, f32, f32, f32, f32, f32)>,
        scissor_rect: Option<(u32, u32, u32, u32)>,
        draw_indirect: (Entity, BufferAddress),
        encoder: Entity,
    ) -> EntityBuilder {
        let mut builder = EntityBuilder::new();

        Self::builder_impl(
            &mut builder,
            order,
            label,
            color_attachments,
            depth_attachment,
            pipeline,
            vertex_buffers,
            index_buffers,
            bind_groups,
            push_constants,
            blend_constant,
            stencil_reference,
            viewport,
            scissor_rect,
            encoder,
        );

        let (indirect_entity, indirect_offset) = draw_indirect;
        let indirect = Indirect::construct(indirect_entity);
        let draw = RenderPassDrawIndirectComponent::construct((indirect, indirect_offset));
        builder.add(draw);

        builder
    }

    pub fn draw_indexed_indirect(
        order: usize,
        label: Option<String>,
        color_attachments: Vec<(Entity, Option<Entity>, Operations<Color>)>,
        depth_attachment: Option<(Entity, Option<Operations<f32>>, Option<Operations<u32>>)>,
        pipeline: Entity,
        vertex_buffers: Vec<(Entity, Range<BufferAddress>)>,
        index_buffers: Option<(Entity, Range<BufferAddress>, IndexFormat)>,
        bind_groups: Vec<(Entity, Vec<DynamicOffset>)>,
        push_constants: Vec<(Entity, ShaderStages)>,
        blend_constant: Option<Color>,
        stencil_reference: Option<u32>,
        viewport: Option<(f32, f32, f32, f32, f32, f32)>,
        scissor_rect: Option<(u32, u32, u32, u32)>,
        draw_indexed_indirect: (Entity, BufferAddress),
        encoder: Entity,
    ) -> EntityBuilder {
        let mut builder = EntityBuilder::new();

        Self::builder_impl(
            &mut builder,
            order,
            label,
            color_attachments,
            depth_attachment,
            pipeline,
            vertex_buffers,
            index_buffers,
            bind_groups,
            push_constants,
            blend_constant,
            stencil_reference,
            viewport,
            scissor_rect,
            encoder,
        );

        let (indirect_entity, indirect_offset) = draw_indexed_indirect;
        let indirect = Indirect::construct(indirect_entity);
        let draw = RenderPassDrawIndexedIndirectComponent::construct((indirect, indirect_offset));
        builder.add(draw);

        builder
    }
}

#[derive(hecs::Query)]
pub struct RenderPassQuery<'a> {
    order: &'a PassOrderComponent,
    label: &'a RenderPassLabelComponent,
    color_attachments: &'a RenderPassColorAttachmentsComponent,
    depth_attachment: &'a RenderPassDepthAttachmentComponent,
    pipeline: &'a RenderPassPipelineComponent,
    vertex_buffers: &'a RenderPassVertexBuffersComponent,
    index_buffer: &'a RenderPassIndexBufferComponent,
    bind_groups: &'a RenderPassBindGroupsComponent,
    push_constants: Option<&'a RenderPassPushConstantsComponent>,
    blend_constant: Option<&'a RenderPassBlendConstantComponent>,
    stencil_reference: Option<&'a RenderPassStencilReferenceComponent>,
    viewport: Option<&'a RenderPassViewportComponent>,
    scissor_rect: Option<&'a RenderPassScissorRectComponent>,
    encoder: &'a RenderPassEncoderComponent,
}

pub fn draw_render_passes_system(world: &mut World) -> Option<()> {
    let mut query = world.query::<RenderPassQuery>();
    let mut components = query.into_iter().collect::<Vec<_>>();
    components.sort_unstable_by(|(_, lhs), (_, rhs)| lhs.order.cmp(rhs.order));

    let mut components = components.into_iter().collect::<Vec<_>>();
    components.sort_unstable_by(
        |(_, RenderPassQuery { order: lhs, .. }), (_, RenderPassQuery { order: rhs, .. })| {
            lhs.cmp(rhs)
        },
    );

    for (
        entity,
        RenderPassQuery {
            label,
            color_attachments,
            depth_attachment,
            pipeline,
            vertex_buffers,
            index_buffer,
            bind_groups,
            push_constants,
            blend_constant,
            stencil_reference,
            viewport,
            scissor_rect,
            encoder,
            ..
        },
    ) in components.into_iter()
    {
        // Collect draw commands
        let mut draw_query = world.query_one::<&RenderPassDrawComponent>(entity).ok();
        let draw = draw_query.as_mut().map(|query| query.get()).flatten();

        let mut draw_indexed_query = world
            .query_one::<&RenderPassDrawIndexedComponent>(entity)
            .ok();
        let draw_indexed = draw_indexed_query
            .as_mut()
            .map(|query| query.get())
            .flatten();

        let mut draw_indirect_query = world
            .query_one::<&RenderPassDrawIndirectComponent>(entity)
            .ok();
        let draw_indirect = draw_indirect_query
            .as_mut()
            .map(|query| query.get())
            .flatten();

        let mut draw_indexed_indirect_query = world
            .query_one::<&RenderPassDrawIndexedIndirectComponent>(entity)
            .ok();
        let draw_indexed_indirect = draw_indexed_indirect_query
            .as_mut()
            .map(|query| query.get())
            .flatten();

        let mut query = encoder.get(world);
        let encoder = query.get().unwrap().get_mut().unwrap();

        // Collect label
        let label = (**label).clone();
        let label = label.as_deref();

        // Collect color attachments
        let mut color_queries = color_attachments
            .iter()
            .map(|(view, resolve_target, ops)| {
                (
                    view.get(world),
                    resolve_target
                        .as_ref()
                        .map(|resolve_target| resolve_target.get(world)),
                    ops,
                )
            })
            .collect::<Vec<_>>();

        let mut color = vec![];
        for (view, resolve_target, ops) in color_queries.iter_mut() {
            let view = view.get().unwrap().get()?;
            let resolve_target = resolve_target
                .as_mut()
                .map(|resolve_target| resolve_target.get().unwrap().get().unwrap());

            color.push((view, resolve_target, ops))
        }

        let color_attachments = color
            .into_iter()
            .map(|(view, resolve_target, ops)| {
                let ops = **ops;

                RenderPassColorAttachment {
                    view,
                    resolve_target,
                    ops,
                }
            })
            .collect::<Vec<_>>();

        // Collect depth stencil attachment
        let mut depth_stencil_query = depth_attachment
            .as_ref()
            .map(|(view, depth_ops, stencil_ops)| (view.get(world), depth_ops, stencil_ops));

        let depth_stencil = depth_stencil_query
            .as_mut()
            .map(|(query, depth_ops, stencil_ops)| {
                (query.get().unwrap().get().unwrap(), depth_ops, stencil_ops)
            });

        let depth_stencil_attachment = depth_stencil.map(|(view, depth_ops, stencil_ops)| {
            let depth_ops = **depth_ops;
            let stencil_ops = **stencil_ops;

            RenderPassDepthStencilAttachment {
                view,
                depth_ops,
                stencil_ops,
            }
        });

        // Collect pipeline
        let mut query = pipeline.get(world);
        let pipeline = query.get()?;
        let pipeline = pipeline.get()?;

        // Collect vertex buffer queries
        let mut vertex_buffer_queries = vertex_buffers
            .iter()
            .map(|(vertex_buffer, range)| (vertex_buffer.get(world), range))
            .collect::<Vec<_>>();

        let vertex_buffer_locks = vertex_buffer_queries
            .iter_mut()
            .map(|(query, range)| {
                let bind_group = query.get().unwrap().read();
                (bind_group, range)
            })
            .collect::<Vec<_>>();

        let vertex_buffers = vertex_buffer_locks
            .iter()
            .map(|(buf, range)| (buf.get().unwrap(), range))
            .collect::<Vec<_>>();

        // Collect index buffer query
        let mut index_buffer_query = index_buffer
            .as_ref()
            .map(|(index_buffer, range, format)| (index_buffer.get(world), range, format));

        let index_buffer_lock = index_buffer_query.as_mut().map(|(query, range, format)| {
            let bind_group = query.get().unwrap().read();
            (bind_group, range, *format)
        });

        let index_buffer = index_buffer_lock
            .as_ref()
            .map(|(lock, range, format)| (lock.get().unwrap(), range, format));

        // Collect bind group queries
        let mut bind_group_queries = bind_groups
            .iter()
            .map(|(bind_group, offsets)| (bind_group.get(world), offsets))
            .collect::<Vec<_>>();

        let bind_groups = bind_group_queries
            .iter_mut()
            .map(|(query, offsets)| {
                let bind_group = query.get().unwrap();
                (bind_group.get().unwrap(), offsets)
            })
            .collect::<Vec<_>>();

        // Collect push constant queries
        let mut push_constant_queries = if let Some(push_constants) = push_constants {
            let push_constant_queries = push_constants
                .iter()
                .map(|(push_constant, shader_stages)| (push_constant.get(world), shader_stages))
                .collect::<Vec<_>>();

            push_constant_queries
        } else {
            vec![]
        };

        let push_constants = push_constant_queries
            .iter_mut()
            .map(|(query, shader_stages)| (query.get().unwrap(), shader_stages))
            .collect::<Vec<_>>();

        // Collect draw indirect query
        let mut indirect_query = draw_indirect.map(|draw_indirect| {
            let (indirect_query, indirect_offset) = &**draw_indirect;
            (indirect_query.get(world), *indirect_offset)
        });

        let draw_indirect_lock =
            indirect_query
                .as_mut()
                .map(|(indirect_query, indirect_offset)| {
                    (indirect_query.get().unwrap().read(), *indirect_offset)
                });

        let draw_indirect = draw_indirect_lock
            .as_ref()
            .map(|(lock, indirect_offset)| (lock.get().unwrap(), *indirect_offset));

        // Collect draw indexed indirect query
        let mut indexed_indirect_query = draw_indexed_indirect.map(|draw_indexed_indirect| {
            let (indirect_query, indirect_offset) = &**draw_indexed_indirect;
            (indirect_query.get(world), *indirect_offset)
        });

        let draw_indexed_indirect_lock =
            indexed_indirect_query
                .as_mut()
                .map(|(indirect_query, indirect_offset)| {
                    (indirect_query.get().unwrap().read(), *indirect_offset)
                });

        let draw_indexed_indirect =
            draw_indexed_indirect_lock
                .as_ref()
                .map(|(indirect_query, indirect_offset)| {
                    (indirect_query.get().unwrap(), *indirect_offset)
                });

        // Begin render pass
        let mut rpass = encoder.begin_render_pass(&RenderPassDescriptor {
            label,
            color_attachments: &color_attachments,
            depth_stencil_attachment,
        });

        rpass.set_pipeline(pipeline);

        // Set vertex buffers
        for (i, (vertex_buffer, range)) in vertex_buffers.iter().enumerate() {
            rpass.set_vertex_buffer(i as u32, vertex_buffer.slice((***range).clone()));
        }

        // Set index buffer
        if let Some((index_buffer, range, format)) = index_buffer {
            rpass.set_index_buffer(index_buffer.slice((*range).clone()), **format);
        }

        // Set bind groups
        for (i, (bind_group, offsets)) in bind_groups.iter().enumerate() {
            rpass.set_bind_group(i as u32, bind_group, &offsets);
        }

        // Set push constants
        for (push_constant, shader_stages) in push_constants {
            rpass.set_push_constants(
                **shader_stages,
                **push_constant.offset,
                &***push_constant.data,
            );
        }

        // Set blend constant
        if let Some(blend_constant) = blend_constant {
            rpass.set_blend_constant(**blend_constant);
        }

        // Set stencil reference
        if let Some(stencil_reference) = stencil_reference {
            rpass.set_stencil_reference(**stencil_reference);
        }

        // Set viewport
        if let Some(viewport) = viewport {
            let (x, y, w, h, min_depth, max_depth) = **viewport;
            rpass.set_viewport(x, y, w, h, min_depth, max_depth);
        }

        // Set scissor_rect
        if let Some(scissor_rect) = scissor_rect {
            let (x, y, w, h) = **scissor_rect;
            rpass.set_scissor_rect(x, y, w, h);
        }

        // Draw
        if let Some(draw) = draw {
            rpass.draw(draw.0.clone(), draw.1.clone());
        }

        // Draw indexed
        if let Some(draw_indexed) = draw_indexed {
            rpass.draw_indexed(
                draw_indexed.0.clone(),
                draw_indexed.1,
                draw_indexed.2.clone(),
            );
        }

        // Draw indirect
        if let Some((indirect_buffer, indirect_offset)) = draw_indirect {
            rpass.draw_indirect(indirect_buffer, indirect_offset);
        }

        // Draw indexed indirect
        if let Some((indirect_buffer, indirect_offset)) = draw_indexed_indirect {
            rpass.draw_indexed_indirect(indirect_buffer, indirect_offset);
        }
    }

    Some(())
}
