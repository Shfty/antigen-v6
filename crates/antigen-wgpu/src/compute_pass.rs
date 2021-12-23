use antigen_core::{AsUsage, Construct, Indirect, Usage};
use hecs::{Entity, EntityBuilder, World};
use wgpu::{ComputePassDescriptor, DynamicOffset};

use crate::{
    BindGroupComponent, CommandEncoderComponent, ComputePipelineComponent, DeviceComponent,
    PushConstantQuery,
};

// TEST: Compute pass automation
pub enum ComputePass {}

pub type ComputePassPipelineComponent =
    Usage<ComputePass, Indirect<&'static ComputePipelineComponent>>;
pub type ComputePassBindGroupsComponent =
    Usage<ComputePass, Vec<(Indirect<&'static BindGroupComponent>, Vec<DynamicOffset>)>>;
pub type ComputePassPushConstantsComponent =
    Usage<ComputePass, Vec<Indirect<PushConstantQuery<'static>>>>;
pub type ComputePassDispatchComponent = Usage<ComputePass, (u32, u32, u32)>;

#[derive(hecs::Bundle)]
pub struct ComputePassBundle {
    pipeline: ComputePassPipelineComponent,
    bind_groups: ComputePassBindGroupsComponent,
    dispatch: ComputePassDispatchComponent,
}

impl ComputePassBundle {
    pub fn builder(
        desc: ComputePassDescriptor<'static>,
        pipeline_entity: Entity,
        bind_group_entities: Vec<(Entity, Vec<DynamicOffset>)>,
        push_constant_entities: Vec<Entity>,
        dispatch: (u32, u32, u32),
    ) -> EntityBuilder {
        let mut builder = EntityBuilder::new();

        builder.add(desc);

        let pipeline = ComputePass::as_usage(Indirect::construct(pipeline_entity));

        let bind_groups = ComputePass::as_usage(
            bind_group_entities
                .into_iter()
                .map(|(entity, offset)| (Indirect::construct(entity), offset))
                .collect::<Vec<_>>(),
        );

        if push_constant_entities.len() > 0 {
            builder.add(ComputePassPushConstantsComponent::construct(
                push_constant_entities
                    .into_iter()
                    .map(Indirect::construct)
                    .collect(),
            ));
        }

        let dispatch = ComputePass::as_usage(dispatch);

        builder.add_bundle(ComputePassBundle {
            pipeline,
            bind_groups,
            dispatch,
        });

        builder
    }
}

#[derive(hecs::Query)]
pub struct ComputePassQuery<'a> {
    desc: &'a ComputePassDescriptor<'static>,
    pipeline: &'a ComputePassPipelineComponent,
    bind_groups: &'a ComputePassBindGroupsComponent,
    push_constants: Option<&'a ComputePassPushConstantsComponent>,
    dispatch: &'a ComputePassDispatchComponent,
    encoder: &'a mut CommandEncoderComponent,
}

pub fn dispatch_compute_passes_system(world: &mut World) -> Option<()> {
    for (
        entity,
        ComputePassQuery {
            desc,
            pipeline,
            bind_groups,
            push_constants,
            dispatch,
            encoder,
        },
    ) in world.query::<ComputePassQuery>().into_iter()
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

        println!(
            "Dispatching compute work groups ({}, {}, {}) for entity {:?}",
            dispatch.0, dispatch.1, dispatch.2, entity
        );
        cpass.dispatch(dispatch.0, dispatch.1, dispatch.2);
    }

    Some(())
}
