use std::{any::TypeId, collections::BTreeMap};

use hecs::{Entity, Ref, RefMut, World};
use usage::Usage;

pub enum TaggedEntity {}
/// Component identifying an entity as a tagged singleton
pub type TaggedEntityComponent = Usage<TaggedEntity, TypeId>;

pub enum TaggedEntities {}
/// TypeId -> Entity map for referring to singletons by tag
pub type TaggedEntitiesComponent = Usage<TaggedEntities, BTreeMap<TypeId, Entity>>;

pub fn get_tagged_entities(
    world: &mut World,
) -> Result<Ref<TaggedEntitiesComponent>, hecs::ComponentError> {
    let query = world.query_mut::<&TaggedEntitiesComponent>();
    let (entity, _) = query
        .into_iter()
        .next()
        .expect("No tagged entities component");
    world.get::<TaggedEntitiesComponent>(entity)
}

pub fn get_tagged_entities_mut(
    world: &mut World,
) -> Result<RefMut<TaggedEntitiesComponent>, hecs::ComponentError> {
    let query = world.query_mut::<&TaggedEntitiesComponent>();
    let (entity, _) = query
        .into_iter()
        .next()
        .expect("No tagged entities component");
    world.get_mut::<TaggedEntitiesComponent>(entity)
}

pub fn get_tagged_entity<T: 'static>(world: &mut World) -> Option<Entity> {
    let type_id = std::any::TypeId::of::<T>();
    let tagged_entities = get_tagged_entities(world)
        .unwrap_or_else(|e| panic!("Error getting entity with tag {:?}: {}", type_id, e));
    tagged_entities.get(&std::any::TypeId::of::<T>()).copied()
}

pub fn insert_tagged_entity<T: 'static>(world: &mut World, entity: Entity) {
    let type_id = std::any::TypeId::of::<T>();
    let mut tagged_entities = get_tagged_entities_mut(world).unwrap();
    tagged_entities.insert(type_id, entity);
}

pub fn insert_tagged_entity_by_query<Q: hecs::Query + Send + Sync + 'static, T: 'static>(
    world: &mut World,
) {
    let (entity, _) = world.query_mut::<Q>().into_iter().next().unwrap();
    insert_tagged_entity::<T>(world, entity);
}
