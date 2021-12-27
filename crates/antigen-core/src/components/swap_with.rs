use std::marker::PhantomData;

use hecs::{Component, EntityBuilder};

use crate::{Construct, Indirect, Usage};

// Swap two components of the same type in-place
#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SwapWith<T>(PhantomData<T>);

impl<T> Default for SwapWith<T> {
    fn default() -> Self {
        SwapWith(Default::default())
    }
}

pub fn swap_with_builder<T: Component>(target_entity: hecs::Entity) -> EntityBuilder {
    let mut builder = EntityBuilder::new();

    builder.add(SwapWith::<T>::default());
    builder.add(Usage::<SwapWith<T>, Indirect<&mut T>>::construct(
        target_entity,
    ));

    builder
}

pub fn swap_with_system<T: Component>(world: &mut hecs::World) {
    let mut query = world
        .query::<(&mut T, &Usage<SwapWith<T>, Indirect<&mut T>>)>()
        .with::<SwapWith<T>>();

    for (_, (component, indirect_component)) in query.into_iter() {
        let mut query = indirect_component.get(world);
        let indirect_buffer = query.get().unwrap();

        std::mem::swap(component, indirect_buffer);
    }
}
