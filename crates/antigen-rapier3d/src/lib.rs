pub use rapier3d;

use antigen_core::{Construct, PositionComponent, RotationComponent, Usage};
use hecs::{Entity, EntityBuilder, Query, World};
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

pub fn physics_backend_builder() -> EntityBuilder {
    let mut builder = EntityBuilder::new();

    builder.add(GravityComponent::construct(
        rapier3d::prelude::nalgebra::Vector3::new(0.0, -9.81, 0.0),
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

pub enum ColliderComponent {
    Pending(Collider),
    PendingParent(Collider, Entity),
    Ready(ColliderHandle),
    Dropped,
}

impl ColliderComponent {
    pub fn new(collider: Collider) -> Self {
        ColliderComponent::Pending(collider)
    }

    pub fn new_with_parent(collider: Collider, parent: Entity) -> Self {
        ColliderComponent::PendingParent(collider, parent)
    }

    fn take_collider(&mut self) -> Option<Collider> {
        match self {
            ColliderComponent::Pending(_) => {
                if let ColliderComponent::Pending(collider) =
                    std::mem::replace(self, ColliderComponent::Dropped)
                {
                    Some(collider)
                } else {
                    None
                }
            }
            _ => None,
        }
    }

    fn take_collider_with_parent(&mut self) -> Option<(Collider, Entity)> {
        match self {
            ColliderComponent::PendingParent(..) => {
                if let ColliderComponent::PendingParent(collider, parent) =
                    std::mem::replace(self, ColliderComponent::Dropped)
                {
                    Some((collider, parent))
                } else {
                    None
                }
            }
            _ => None,
        }
    }
}

pub fn insert_colliders_system(world: &mut World) {
    let mut query = world.query::<(&mut ColliderSet, &mut RigidBodySet)>();
    let (_, (collider_set, rigid_body_set)) = query.into_iter().next().unwrap();

    for (_, collider) in world.query::<&mut ColliderComponent>().into_iter() {
        if let Some(c) = collider.take_collider() {
            let handle = collider_set.insert(c);
            *collider = ColliderComponent::Ready(handle);
        }

        if let Some((c, parent)) = collider.take_collider_with_parent() {
            let mut query = world.query_one::<&RigidBodyComponent>(parent).unwrap();
            let parent = query.get().unwrap();
            if let RigidBodyComponent::Ready(parent) = parent {
                let handle = collider_set.insert_with_parent(c, *parent, rigid_body_set);
                *collider = ColliderComponent::Ready(handle);
            }
        }
    }
}

pub enum RigidBodyComponent {
    Pending(RigidBody),
    Ready(RigidBodyHandle),
    Dropped,
}

impl RigidBodyComponent {
    pub fn new(collider: RigidBody) -> Self {
        RigidBodyComponent::Pending(collider)
    }

    fn take_rigid_body(&mut self) -> Option<RigidBody> {
        match self {
            RigidBodyComponent::Pending(_) => {
                if let RigidBodyComponent::Pending(rigid_body) =
                    std::mem::replace(self, RigidBodyComponent::Dropped)
                {
                    Some(rigid_body)
                } else {
                    None
                }
            }
            _ => None,
        }
    }
}

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
        if let Some(mut rb) = rigid_body.take_rigid_body() {
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
            *rigid_body = RigidBodyComponent::Ready(handle);
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
        if let RigidBodyComponent::Ready(handle) = rigid_body {
            let rb = &rigid_body_set[*handle];

            if let Some(position) = position {
                let pos = rb.translation();
                println!("Position: {}", pos);
                **position = nalgebra::vector![pos.x, pos.y, pos.z];
            }

            if let Some(rotation) = rotation {
                let rot = rb.rotation();
                println!("Rotation: {}", rot);
                let (x, y, z) = rot.euler_angles();
                **rotation = nalgebra::UnitQuaternion::from_euler_angles(x, y, z);
            }
        }
    }
}
