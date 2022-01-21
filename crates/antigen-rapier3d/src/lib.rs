pub use rapier3d;

use antigen_core::{
    Construct, Indirect, LazyComponent, PositionComponent, RotationComponent, Usage,
};
use hecs::{EntityBuilder, Query, World};
use rapier3d::prelude::{
    BroadPhase, CCDSolver, Collider, ColliderHandle, ColliderSet, IntegrationParameters,
    IslandManager, JointSet, NarrowPhase, PhysicsPipeline, RigidBody, RigidBodyHandle,
    RigidBodySet,
};

pub enum Gravity {}

pub type GravityComponent = Usage<Gravity, rapier3d::prelude::nalgebra::Vector3<f32>>;

#[derive(Query)]
pub struct PhysicsQuery<'a> {
    pub gravity: &'a GravityComponent,
    pub integration_parameters: &'a IntegrationParameters,
    pub physics_pipeline: &'a mut PhysicsPipeline,
    pub island_manager: &'a mut IslandManager,
    pub broad_phase: &'a mut BroadPhase,
    pub narrow_phase: &'a mut NarrowPhase,
    pub rigid_body_set: &'a mut RigidBodySet,
    pub collider_set: &'a mut ColliderSet,
    pub joint_set: &'a mut JointSet,
    pub ccd_solver: &'a mut CCDSolver,
}

pub fn physics_backend_builder(gravity: nalgebra::Vector3<f32>) -> EntityBuilder {
    let mut builder = EntityBuilder::new();

    builder.add(GravityComponent::construct(
        rapier3d::prelude::nalgebra::Vector3::new(gravity.x, gravity.y, gravity.z),
    ));
    builder.add(IntegrationParameters::default());
    builder.add(PhysicsPipeline::default());
    builder.add(IslandManager::new());
    builder.add(BroadPhase::new());
    builder.add(NarrowPhase::new());
    builder.add(RigidBodySet::new());
    builder.add(ColliderSet::new());
    builder.add(JointSet::new());
    builder.add(CCDSolver::new());

    builder
}

pub fn step_physics_system(world: &mut World) {
    for (
        _,
        PhysicsQuery {
            gravity,
            integration_parameters,
            physics_pipeline,
            island_manager,
            broad_phase,
            narrow_phase,
            rigid_body_set,
            collider_set,
            joint_set,
            ccd_solver,
        },
    ) in world.query_mut::<PhysicsQuery>().into_iter()
    {
        physics_pipeline.step(
            &gravity,
            integration_parameters,
            island_manager,
            broad_phase,
            narrow_phase,
            rigid_body_set,
            collider_set,
            joint_set,
            ccd_solver,
            &(),
            &(),
        );
    }
}

pub type ColliderComponent = LazyComponent<ColliderHandle, Collider>;

pub enum ColliderParent {}
pub type ColliderParentComponent<'a> = Usage<ColliderParent, Indirect<&'a RigidBodyComponent>>;

pub fn insert_colliders_system(world: &mut World) {
    let mut query = world.query::<(&mut ColliderSet, &mut RigidBodySet)>();
    let (_, (collider_set, rigid_body_set)) = query.into_iter().next().unwrap();

    for (_, components) in world
        .query::<(&mut ColliderComponent, Option<&ColliderParentComponent>)>()
        .into_iter()
    {
        match components {
            (collider @ ColliderComponent::Pending(_), None) => {
                let c = if let LazyComponent::Pending(c) = collider.take() {
                    c
                } else {
                    panic!("No collider component")
                };
                let handle = collider_set.insert(c);
                *collider = ColliderComponent::Ready(handle);
            }
            (collider @ ColliderComponent::Pending(_), Some(parent)) => {
                let mut query = parent.get(world);
                let parent = query.get().unwrap();
                if let LazyComponent::Ready(parent) = **parent {
                    let c = if let LazyComponent::Pending(c) = collider.take() {
                        c
                    } else {
                        panic!("No collider component")
                    };
                    let handle = collider_set.insert_with_parent(c, parent, rigid_body_set);
                    *collider = ColliderComponent::Ready(handle);
                }
            }
            _ => (),
        }
    }
}

pub enum RigidBodyTag {}
pub type RigidBodyComponent = Usage<RigidBodyTag, LazyComponent<RigidBodyHandle, RigidBody>>;

pub fn insert_rigid_bodies_system(world: &mut World) {
    let mut query = world.query::<&mut RigidBodySet>();
    let (_, rigid_body_set) = query.into_iter().next().unwrap();

    for (_, (rigid_body, position, rotation)) in world
        .query::<(
            &mut RigidBodyComponent,
            Option<&PositionComponent>,
            Option<&RotationComponent>,
        )>()
        .into_iter()
    {
        if let LazyComponent::Pending(_) = **rigid_body {
            let mut rb = if let LazyComponent::Pending(rb) = rigid_body.take() {
                rb
            } else {
                panic!("No collider component")
            };

            if let Some(position) = position {
                let pos =
                    rapier3d::prelude::nalgebra::Vector3::new(position.x, position.y, position.z);
                rb.set_translation(pos, false);
            }
            if let Some(rotation) = rotation {
                let (x, y, z) = rotation.euler_angles();
                rb.set_rotation(rapier3d::prelude::AngVector::new(x, y, z), false);
            }
            let handle = rigid_body_set.insert(rb);
            **rigid_body = LazyComponent::Ready(handle);
        }
    }
}

pub fn read_back_rigid_body_isometries_system(world: &mut World) {
    let mut query = world.query::<&mut RigidBodySet>();
    let (_, rigid_body_set) = query.into_iter().next().unwrap();

    for (_, (rigid_body, position, rotation)) in world
        .query::<(
            &RigidBodyComponent,
            Option<&mut PositionComponent>,
            Option<&mut RotationComponent>,
        )>()
        .into_iter()
    {
        if let LazyComponent::Ready(handle) = **rigid_body {
            let rb = &rigid_body_set[handle];

            if let Some(position) = position {
                let pos = rb.translation();
                **position = nalgebra::vector![pos.x, pos.y, pos.z];
            }

            if let Some(rotation) = rotation {
                let rot = rb.rotation();
                let (x, y, z) = rot.euler_angles();
                **rotation = nalgebra::UnitQuaternion::from_euler_angles(x, y, z);
            }
        }
    }
}
