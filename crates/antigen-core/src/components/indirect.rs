use std::marker::PhantomData;

use hecs::{Component, Entity, Fetch, Query, QueryOne, World};

use crate::{peano::Z, Construct};

pub struct Indirect<T> {
    entity: Entity,
    _phantom: PhantomData<T>,
}

impl<T> Construct<Entity, Z> for Indirect<T> {
    fn construct(entity: Entity) -> Self {
        Indirect {
            entity,
            _phantom: Default::default(),
        }
    }
}

impl<T> Indirect<T>
where
    T: Query + Component,
{
    pub fn entity(&self) -> Entity {
        self.entity
    }

    pub fn get<'a>(&self, world: &'a World) -> QueryOne<'a, T> {
        world.query_one::<T>(self.entity).unwrap()
    }

    pub fn get_mut<'a>(&self, world: &'a mut World) -> <<T as Query>::Fetch as Fetch<'a>>::Item {
        world.query_one_mut::<T>(self.entity).unwrap()
    }
}
