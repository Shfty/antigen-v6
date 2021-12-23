use std::marker::PhantomData;

use hecs::{Component, Entity, QueryOne, World};

use crate::{Construct, peano::Z};

pub struct Indirect<T> {
    entity: Entity,
    _phantom: PhantomData<T>
}

impl<T> Construct<Entity, Z> for Indirect<T> {
    fn construct(entity: Entity) -> Self {
        Indirect {
            entity,
            _phantom: Default::default(),
        }
    }
}

impl<T> Indirect<T> where T: Component {
    pub fn get<'a>(&self, world: &'a World) -> QueryOne<'a, &T> {
        world.query_one::<&T>(self.entity).unwrap()
    }

    pub fn get_mut<'a>(&self, world: &'a World) -> QueryOne<'a, &mut T> {
        world.query_one::<&mut T>(self.entity).unwrap()
    }

    pub fn get_unique<'a>(&self, world: &'a mut World) -> &'a T {
        world.query_one_mut::<&T>(self.entity).unwrap()
    }

    pub fn get_unique_mut<'a>(&self, world: &'a mut World) -> &'a mut T {
        world.query_one_mut::<&mut T>(self.entity).unwrap()
    }
}
