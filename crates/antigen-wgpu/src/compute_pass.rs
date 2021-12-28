use std::collections::BTreeMap;

use antigen_core::{AsUsage, Construct, Indirect, Usage};
use hecs::{Entity, EntityBuilder, World};
use wgpu::{BufferAddress, ComputePassDescriptor, DynamicOffset};

use crate::{BindGroupComponent, BufferComponent, CommandEncoderComponent, ComputePipelineComponent, PassOrderComponent, PushConstantQuery};

pub enum ComputePassTag {}

pub type ComputePassPipelineComponent =
    Usage<ComputePassTag, Indirect<&'static ComputePipelineComponent>>;
pub type ComputePassBindGroupsComponent =
    Usage<ComputePassTag, Vec<(Indirect<&'static BindGroupComponent>, Vec<DynamicOffset>)>>;
pub type ComputePassPushConstantsComponent =
    Usage<ComputePassTag, Vec<Indirect<PushConstantQuery<'static>>>>;
pub type ComputePassDispatchComponent = Usage<ComputePassTag, (u32, u32, u32)>;

pub struct ComputePassDispatchIndirectComponent {
    buffer: Indirect<&'static BufferComponent>,
    offset: BufferAddress,
}

pub enum ComputePassBundle {}

fn compute_pass_bundle_impl(
    builder: &mut EntityBuilder,
    order: usize,
    desc: ComputePassDescriptor<'static>,
    pipeline_entity: Entity,
    bind_group_entities: Vec<(Entity, Vec<DynamicOffset>)>,
    push_constant_entities: Vec<Entity>,
) {
    builder.add(PassOrderComponent::construct(order));

    builder.add(desc);

    let pipeline = ComputePassPipelineComponent::construct(pipeline_entity);
    builder.add(pipeline);

    let bind_groups = ComputePassTag::as_usage(
        bind_group_entities
            .into_iter()
            .map(|(entity, offset)| {
                (
                    Indirect::<&'static BindGroupComponent>::construct(entity),
                    offset,
                )
            })
            .collect::<Vec<_>>(),
    );

    builder.add(bind_groups);

    if push_constant_entities.len() > 0 {
        builder.add(ComputePassPushConstantsComponent::construct(
            push_constant_entities
                .into_iter()
                .map(Indirect::construct)
                .collect(),
        ));
    }
}

impl ComputePassBundle {
    pub fn dispatch(
        order: usize,
        desc: ComputePassDescriptor<'static>,
        pipeline_entity: Entity,
        bind_group_entities: Vec<(Entity, Vec<DynamicOffset>)>,
        push_constant_entities: Vec<Entity>,
        dispatch: (u32, u32, u32),
    ) -> EntityBuilder {
        let mut builder = EntityBuilder::new();

        compute_pass_bundle_impl(
            &mut builder,
            order,
            desc,
            pipeline_entity,
            bind_group_entities,
            push_constant_entities,
        );

        let dispatch = ComputePassTag::as_usage(dispatch);
        builder.add(dispatch);

        builder
    }

    pub fn dispatch_indirect(
        order: usize,
        desc: ComputePassDescriptor<'static>,
        pipeline_entity: Entity,
        bind_group_entities: Vec<(Entity, Vec<DynamicOffset>)>,
        push_constant_entities: Vec<Entity>,
        indirect_entity: Entity,
        indirect_offset: BufferAddress,
    ) -> EntityBuilder {
        let mut builder = EntityBuilder::new();

        compute_pass_bundle_impl(
            &mut builder,
            order,
            desc,
            pipeline_entity,
            bind_group_entities,
            push_constant_entities,
        );

        let buffer = Indirect::construct(indirect_entity);
        let offset = indirect_offset;

        let dispatch = ComputePassDispatchIndirectComponent { buffer, offset };
        builder.add(dispatch);

        builder
    }
}

#[derive(hecs::Query)]
pub struct ComputePassQuery<'a> {
    order: &'a PassOrderComponent,
    desc: &'a ComputePassDescriptor<'static>,
    pipeline: &'a ComputePassPipelineComponent,
    bind_groups: &'a ComputePassBindGroupsComponent,
    push_constants: Option<&'a ComputePassPushConstantsComponent>,
    dispatch: hecs::Or<&'a ComputePassDispatchComponent, &'a ComputePassDispatchIndirectComponent>,
    encoder: &'a mut CommandEncoderComponent,
}

pub fn dispatch_compute_passes_system(world: &mut World) -> Option<()> {
    let mut query = world.query::<ComputePassQuery>();

    let mut components = query.into_iter().collect::<Vec<_>>();
    components.sort_unstable_by(|(_, lhs), (_, rhs)| lhs.order.cmp(rhs.order));

    for (
        entity,
        ComputePassQuery {
            desc,
            pipeline,
            bind_groups,
            push_constants,
            dispatch,
            encoder,
            ..
        },
    ) in components.into_iter()
    {
        let encoder = encoder.get_mut()?;

        // Collect pipeline
        let mut query = pipeline.get(world);
        let pipeline = query.get()?;
        let pipeline = pipeline.get()?;

        // Collect bind group queries
        let (mut bind_group_queries, bind_group_offsets): (Vec<_>, Vec<_>) = bind_groups
            .iter()
            .map(|(bind_group, offsets)| (bind_group.get(world), offsets))
            .unzip();

        let bind_groups = bind_group_queries
            .iter_mut()
            .map(|query| {
                let bind_group = query.get().unwrap();
                bind_group.get().unwrap()
            })
            .collect::<Vec<_>>();

        // Collect push constant queries
        let mut push_constant_queries = if let Some(push_constants) = push_constants {
            let push_constant_queries = push_constants
                .iter()
                .map(|push_constant| push_constant.get(world))
                .collect::<Vec<_>>();

            push_constant_queries
        } else {
            vec![]
        };

        let push_constants = push_constant_queries
            .iter_mut()
            .map(|query| query.get().unwrap())
            .collect::<Vec<_>>();

        let dispatch_ind = dispatch.right();
        let mut dispatch_ind_query =
            dispatch_ind.map(|dispatch_ind| (dispatch_ind.buffer.get(world), dispatch_ind.offset));
        let dispatch_ind_buffer = dispatch_ind_query
            .as_mut()
            .map(|(query, offset)| (query.get().unwrap(), *offset));

        let dispatch = dispatch.left();

        let mut cpass = encoder.begin_compute_pass(&desc);
        println!("Setting pipeline {:?}", pipeline);
        cpass.set_pipeline(pipeline);

        for (i, (bind_group, offsets)) in bind_groups
            .iter()
            .zip(bind_group_offsets.iter())
            .enumerate()
        {
            println!(
                "Setting bind group {}: {:?} with offsets {:?}",
                i as u32, bind_group, offsets
            );
            cpass.set_bind_group(i as u32, bind_group, &offsets);
        }

        for push_constant in push_constants {
            println!(
                "Setting push constant with offset {}",
                **push_constant.offset
            );
            cpass.set_push_constants(**push_constant.offset, &***push_constant.data);
        }

        if let Some(dispatch) = dispatch {
            println!(
                "Dispatching compute work groups ({}, {}, {}) for entity {:?}",
                dispatch.0, dispatch.1, dispatch.2, entity
            );
            cpass.dispatch(dispatch.0, dispatch.1, dispatch.2);
        }

        if let Some((buffer, offset)) = dispatch_ind_buffer {
            println!(
                "Dispatching indirect compute work group for entity {:?}",
                entity
            );
            let buffer = buffer.get().unwrap();
            cpass.dispatch_indirect(buffer, offset);
        }
    }

    Some(())
}
