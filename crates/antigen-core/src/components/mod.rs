mod args;
mod changed;
mod indirect;
mod lazy_component;
mod swap_with;
mod tagged_entities;
mod named_entities;
mod usage;

pub use ::usage::*;
pub use args::*;
pub use changed::*;
pub use indirect::*;
pub use lazy_component::*;
pub use swap_with::*;
pub use tagged_entities::*;
pub use named_entities::*;

// Position
pub enum Position {}
pub type PositionComponent = Usage<Position, nalgebra::Vector3<f32>>;

// Rotation
pub enum Rotation {}
pub type RotationComponent = Usage<Rotation, nalgebra::UnitQuaternion<f32>>;

// Scale
pub enum Scale {}
pub type ScaleComponent = Usage<Scale, nalgebra::Vector3<f32>>;

pub enum CopyTo {}
pub type CopyToComponent<'a, U, T> = Usage<U, IndirectMulti<&'a mut Changed<T>>>;

pub fn copy_to_system<U: hecs::Component, T: hecs::Component + PartialEq + Copy>(
    world: &mut hecs::World,
) {
    for (_, (value, copy_to)) in world.query::<(&T, &CopyToComponent<U, T>)>().into_iter() {
        for target in copy_to.entities() {
            let mut query = world.query_one::<&mut Changed<T>>(*target).unwrap();
            let target = query.get().unwrap();
            if **target != *value {
                **target = *value;
                target.set_changed(true);
            }
        }
    }
}
