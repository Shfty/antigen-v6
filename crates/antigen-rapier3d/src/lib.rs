use parking_lot::{RwLock, RwLockReadGuard};

pub use rapier3d;

use antigen_core::{
    Construct, Indirect, LazyComponent, PositionComponent, RotationComponent, Usage,
};
use hecs::{EntityBuilder, Query, World};
use rapier3d::{
    pipeline::EventHandler,
    prelude::{
        BroadPhase, CCDSolver, Collider, ColliderHandle, ColliderSet, ContactEvent, ContactPair,
        IntegrationParameters, IntersectionEvent, IslandManager, JointSet, NarrowPhase,
        PhysicsPipeline, RigidBody, RigidBodyHandle, RigidBodySet, RigidBodyType,
    },
};

// Gravity
pub enum Gravity {}
pub type GravityComponent = Usage<Gravity, rapier3d::prelude::nalgebra::Vector3<f32>>;

// Linear Velocity
pub enum LinearVelocity {}
pub type LinearVelocityComponent = Usage<LinearVelocity, nalgebra::Vector3<f32>>;

// Angular Velocity
pub enum AngularVelocity {}
pub type AngularVelocityComponent = Usage<AngularVelocity, nalgebra::Vector3<f32>>;

// Event Handler
#[derive(Default)]
pub struct EventCollector {
    pub intersection_events: parking_lot::RwLock<Vec<IntersectionEvent>>,
    pub contact_events: parking_lot::RwLock<Vec<(ContactEvent, ContactPair)>>,
}

impl EventHandler for EventCollector {
    fn handle_intersection_event(&self, event: IntersectionEvent) {
        self.intersection_events.write().push(event);
    }

    fn handle_contact_event(
        &self,
        event: rapier3d::prelude::ContactEvent,
        contact_pair: &rapier3d::prelude::ContactPair,
    ) {
        self.contact_events
            .write()
            .push((event, contact_pair.clone()));
    }
}

impl EventCollector {
    pub fn intersection_events(&self) -> RwLockReadGuard<Vec<IntersectionEvent>> {
        self.intersection_events.read()
    }

    pub fn contact_events(&self) -> RwLockReadGuard<Vec<(ContactEvent, ContactPair)>> {
        self.contact_events.read()
    }

    pub fn clear(&self) {
        self.intersection_events.write().clear();
        self.contact_events.write().clear();
    }
}

// Physics backend
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
    pub event_collector: &'a EventCollector,
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
    builder.add(EventCollector::default());

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
            event_collector,
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
            event_collector,
        );
    }
}

pub fn clear_physics_events_system(world: &mut World) {
    for (_, event_collector) in world.query_mut::<&EventCollector>().into_iter() {
        event_collector.clear()
    }
}

pub type ColliderComponent = LazyComponent<ColliderHandle, Collider>;

pub enum ColliderParent {}
pub type ColliderParentComponent<'a> = Usage<ColliderParent, Indirect<&'a RigidBodyComponent>>;

pub fn insert_colliders_system(world: &mut World) {
    let mut query = world.query::<(&mut ColliderSet, &mut RigidBodySet)>();
    let (_, (collider_set, rigid_body_set)) = query.into_iter().next().unwrap();

    for (_, (collider_component, position, rotation, rigid_body, collider_parent)) in world
        .query::<(
            &mut ColliderComponent,
            Option<&PositionComponent>,
            Option<&RotationComponent>,
            Option<&RigidBodyComponent>,
            Option<&ColliderParentComponent>,
        )>()
        .into_iter()
    {
        if let ColliderComponent::Pending(collider) = collider_component {
            // If not attached to a rigidbody, apply position / rotation directly
            if rigid_body.is_none() {
                if let Some(position) = position {
                    collider.set_translation(rapier3d::prelude::nalgebra::Vector3::new(
                        position.x, position.y, position.z,
                    ));
                }

                if let Some(rotation) = rotation {
                    let (x, y, z) = rotation.euler_angles();
                    collider.set_rotation(rapier3d::prelude::nalgebra::Vector3::new(x, y, z));
                }
            }

            match (rigid_body, collider_parent) {
                (None, None) => {
                    let c = if let LazyComponent::Pending(c) = collider_component.take() {
                        c
                    } else {
                        panic!("No collider component")
                    };
                    let handle = collider_set.insert(c);
                    *collider_component = ColliderComponent::Ready(handle);
                }
                (Some(rigid_body), _) => {
                    if let LazyComponent::Ready(rb) = **rigid_body {
                        let c = if let LazyComponent::Pending(c) = collider_component.take() {
                            c
                        } else {
                            panic!("No collider component")
                        };
                        let handle = collider_set.insert_with_parent(c, rb, rigid_body_set);
                        *collider_component = ColliderComponent::Ready(handle);
                    }
                }
                (None, Some(parent)) => {
                    let mut query = parent.get(world);
                    let parent = query.get().unwrap();
                    if let LazyComponent::Ready(parent) = **parent {
                        let c = if let LazyComponent::Pending(c) = collider_component.take() {
                            c
                        } else {
                            panic!("No collider component")
                        };
                        let handle = collider_set.insert_with_parent(c, parent, rigid_body_set);
                        *collider_component = ColliderComponent::Ready(handle);
                    }
                }
            }
        }
    }
}

pub enum RigidBodyTag {}
pub type RigidBodyComponent = Usage<RigidBodyTag, LazyComponent<RigidBodyHandle, RigidBody>>;

pub fn insert_rigid_bodies_system(world: &mut World) {
    let mut query = world.query::<&mut RigidBodySet>();
    let (_, rigid_body_set) = query.into_iter().next().unwrap();

    for (_, (rigid_body, position, rotation, linear_velocity, angular_velocity)) in world
        .query::<(
            &mut RigidBodyComponent,
            Option<&PositionComponent>,
            Option<&RotationComponent>,
            Option<&LinearVelocityComponent>,
            Option<&AngularVelocityComponent>,
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

            if let Some(linear_velocity) = linear_velocity {
                let vel = rapier3d::prelude::nalgebra::Vector3::new(
                    linear_velocity.x,
                    linear_velocity.y,
                    linear_velocity.z,
                );
                rb.set_linvel(vel, false);
            }

            if let Some(angular_velocity) = angular_velocity {
                let vel = rapier3d::prelude::nalgebra::Vector3::new(
                    angular_velocity.x,
                    angular_velocity.y,
                    angular_velocity.z,
                );
                rb.set_angvel(vel, false);
            }

            let handle = rigid_body_set.insert(rb);
            **rigid_body = LazyComponent::Ready(handle);
        }
    }
}

pub fn write_rigid_body_isometries_system(world: &mut World) {
    let mut query = world.query::<&mut RigidBodySet>();
    let (_, rigid_body_set) = query.into_iter().next().unwrap();

    for (_, (rigid_body, position, rotation, linear_velocity, angular_velocity)) in world
        .query::<(
            &mut RigidBodyComponent,
            Option<&mut PositionComponent>,
            Option<&mut RotationComponent>,
            Option<&mut LinearVelocityComponent>,
            Option<&mut AngularVelocityComponent>,
        )>()
        .into_iter()
    {
        if let LazyComponent::Ready(handle) = **rigid_body {
            let rb = &mut rigid_body_set[handle];

            match rb.body_type() {
                RigidBodyType::Dynamic => continue,
                RigidBodyType::Static => continue,
                RigidBodyType::KinematicPositionBased => {
                    if let Some(position) = position {
                        rb.set_next_kinematic_translation(
                            rapier3d::prelude::nalgebra::Vector3::new(
                                position.x, position.y, position.z,
                            ),
                        );
                    }

                    if let Some(rotation) = rotation {
                        let (x, y, z) = rotation.euler_angles();
                        rb.set_next_kinematic_rotation(rapier3d::prelude::nalgebra::Vector3::new(
                            x, y, z,
                        ));
                    }
                }
                RigidBodyType::KinematicVelocityBased => {
                    if let Some(linear_velocity) = linear_velocity {
                        rb.set_linvel(rapier3d::prelude::nalgebra::Vector3::new(
                            linear_velocity.x,
                            linear_velocity.y,
                            linear_velocity.z,
                        ), linear_velocity.magnitude() > 0.0);
                    }

                    if let Some(angular_velocity) = angular_velocity {
                        rb.set_angvel(rapier3d::prelude::nalgebra::Vector3::new(
                            angular_velocity.x,
                            angular_velocity.y,
                            angular_velocity.z,
                        ), angular_velocity.magnitude() > 0.0);
                    }
                }
            }
        }
    }
}

pub fn read_back_rigid_body_isometries_system(world: &mut World) {
    let mut query = world.query::<&mut RigidBodySet>();
    let (_, rigid_body_set) = query.into_iter().next().unwrap();

    for (_, (rigid_body, position, rotation, linear_velocity, angular_velocity)) in world
        .query::<(
            &RigidBodyComponent,
            Option<&mut PositionComponent>,
            Option<&mut RotationComponent>,
            Option<&mut LinearVelocityComponent>,
            Option<&mut AngularVelocityComponent>,
        )>()
        .into_iter()
    {
        if let LazyComponent::Ready(handle) = **rigid_body {
            let rb = &rigid_body_set[handle];

            if rb.body_type() != RigidBodyType::Dynamic {
                continue;
            }

            if let Some(position) = position {
                let pos = rb.translation();
                **position = nalgebra::vector![pos.x, pos.y, pos.z];
            }

            if let Some(rotation) = rotation {
                let rot = rb.rotation();
                let (x, y, z) = rot.euler_angles();
                **rotation = nalgebra::UnitQuaternion::from_euler_angles(x, y, z);
            }

            if let Some(linear_velocity) = linear_velocity {
                let vel = rb.linvel();
                **linear_velocity = nalgebra::vector![vel.x, vel.y, vel.z];
            }

            if let Some(angular_velocity) = angular_velocity {
                let vel = rb.angvel();
                **angular_velocity = nalgebra::vector![vel.x, vel.y, vel.z];
            }
        }
    }
}
