use usage::Usage;

pub enum TaggedEntities {}

/// TypeId -> Entity map for referring to singletons by tag type
pub type TaggedEntitiesComponent =
    Usage<TaggedEntities, std::collections::BTreeMap<std::any::TypeId, hecs::Entity>>;
