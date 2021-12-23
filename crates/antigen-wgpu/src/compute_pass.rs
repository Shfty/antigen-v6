// TEST: Compute pass automation
pub enum PushConstant {}
pub type PushConstantComponent = Usage<PushConstant, (u32, Vec<u8>)>;

pub enum ComputePass {}

pub type ComputePassLabelComponent = Usage<ComputePass, String>;
pub type ComputePassPipelineComponent = Usage<ComputePass, Indirect<ComputePipelineComponent>>;
pub type ComputePassBindGroupsComponent =
    Usage<ComputePass, Vec<(Indirect<BindGroupComponent>, Vec<DynamicOffset>)>>;
pub type ComputePassPushConstantsComponent =
    Usage<ComputePass, Vec<Indirect<PushConstantComponent>>>;
pub type ComputePassDispatchComponent = Usage<ComputePass, (u32, u32, u32)>;

#[derive(hecs::Bundle)]
pub struct ComputePassBundle {
    pipeline: ComputePassPipelineComponent,
    bind_groups: ComputePassBindGroupsComponent,
    dispatch: ComputePassDispatchComponent,
}

impl ComputePassBundle {
    pub fn builder(
        label: Option<String>,
        pipeline_entity: Entity,
        bind_group_entities: Vec<(Entity, BufferAddress)>,
        push_constant_entities: Vec<Entity>,
        dispatch: (u32, u32, u32),
    ) -> EntityBuilder {
        let mut builder = EntityBuilder::new();

        let pipeline = Indirect::construct(pipeline_entity);

        let bind_groups = bind_group_entities
            .into_iter()
            .map(|(entity, offset)| (Indirect::construct(entity), offset))
            .collect::<Vec<_>>();

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
    label: Option<&'a ComputePassLabelComponent>,
    pipeline: &'a ComputePassPipelineComponent,
    bind_groups: &'a ComputePassBindGroupsComponent,
    push_constants: Option<&'a ComputePassPushConstantsComponent>,
    dispatch: &'a ComputePassDispatchComponent,
}

pub fn dispatch_compute_pass_system(world: &mut World) -> Option<()> {
    let mut query = world.query::<&DeviceComponent>();
    let (_, device) = query.into_iter().next()?;
    let device = device.clone();
    drop(query);

    for (
        _,
        ComputePassQuery {
            label,
            pipeline,
            bind_groups,
            push_constants,
            dispatch,
        },
    ) in world.query::<ComputePassQuery>().into_iter()
    {
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

        let mut encoder = device.create_command_encoder(&CommandEncoderDescriptor { label: None });
        let mut cpass = encoder.begin_compute_pass(&ComputePassDescriptor {
            label: label.map(|label| label.as_str()),
        });
        cpass.set_pipeline(pipeline);

        for (i, (bind_group, offsets)) in bind_groups
            .iter()
            .zip(bind_group_offsets.iter())
            .enumerate()
        {
            cpass.set_bind_group(i as u32, bind_group, &offsets);
        }

        for push_constant in push_constants {
            cpass.set_push_constants(push_constant.0, &push_constant.1);
        }

        cpass.dispatch(dispatch.0, dispatch.1, dispatch.2);
    }

    Some(())
}
