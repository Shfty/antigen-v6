use std::{
    borrow::Cow,
    collections::{BTreeMap, BTreeSet},
};

use hecs::{Entity, Ref, RefMut, World};
use usage::Usage;

pub enum NamedEntity {}
/// Component identifying an entity as a tagged singleton
pub type NamedEntityComponent = Usage<NamedEntity, Cow<'static, str>>;

pub enum NamedEntities {}
/// TypeId -> Entity map for referring to singletons by tag
pub type NamedEntitiesComponent =
    Usage<NamedEntities, BTreeMap<Cow<'static, str>, BTreeSet<Entity>>>;

pub fn get_named_entities_component(
    world: &World,
) -> Result<Ref<NamedEntitiesComponent>, hecs::ComponentError> {
    let mut query = world.query::<&NamedEntitiesComponent>();
    let (entity, _) = query
        .into_iter()
        .next()
        .expect("No tagged entities component");
    world.get::<NamedEntitiesComponent>(entity)
}

pub fn get_named_entities_component_mut(
    world: &World,
) -> Result<RefMut<NamedEntitiesComponent>, hecs::ComponentError> {
    let entity = {
        let mut query = world.query::<&NamedEntitiesComponent>();
        query
            .into_iter()
            .next()
            .expect("No tagged entities component")
            .0
    };
    world.get_mut::<NamedEntitiesComponent>(entity)
}

pub fn insert_named_entity(world: &mut World, name: Cow<'static, str>, entity: Entity) {
    let mut tagged_entities = get_named_entities_component_mut(world).unwrap();
    tagged_entities.entry(name).or_default().insert(entity);
}

pub fn insert_named_entities_system(world: &mut World) {
    let mut named_entities = get_named_entities_component_mut(world).unwrap();

    let mut query = world.query::<&NamedEntityComponent>();
    for (entity, name) in query.into_iter() {
        named_entities
            .entry((**name).clone())
            .or_default()
            .insert(entity);
    }
}
