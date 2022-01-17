mod changed;
mod lazy_component;
mod usage;
mod args;
mod indirect;
mod swap_with;
mod tagged_entities;

pub use changed::*;
pub use lazy_component::*;
pub use ::usage::*;
pub use args::*;
pub use indirect::*;
pub use swap_with::*;
pub use tagged_entities::*;

// Position
pub enum Position {}
pub type PositionComponent = Usage<Position, nalgebra::Vector3<f32>>;

// Rotation
pub enum Rotation {}
pub type RotationComponent = Usage<Rotation, nalgebra::UnitQuaternion<f32>>;

// Scale
pub enum Scale {}
pub type ScaleComponent = Usage<Scale, nalgebra::Vector3<f32>>;
