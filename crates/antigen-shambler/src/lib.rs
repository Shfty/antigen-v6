pub use shambler;

use std::path::PathBuf;

use antigen_core::{Construct, MessageContext, MessageResult, Usage};
use antigen_fs::{FilePathComponent, FileStringQuery};
use shambler::GeoMap;

pub enum MapFile {}
pub type MapFileComponent = Usage<MapFile, GeoMap>;

#[derive(hecs::Query)]
pub struct MapFileQuery<'a> {
    pub path: &'a FilePathComponent,
    pub map: &'a MapFileComponent,
}

/// Find a file entity with a matching path and parse it into a GeoMap
pub fn parse_map_file_string<'a, 'b, P: Into<PathBuf>>(
    path: P,
) -> impl FnOnce(MessageContext<'a, 'b>) -> MessageResult<'a, 'b> {
    move |mut ctx| {
        let (world, _) = &mut ctx;

        let map_path = path.into();
        println!(
            "Thread {} Looking for file string entities with path {:?}..",
            std::thread::current().name().unwrap(),
            map_path
        );

        let components = world
            .query_mut::<FileStringQuery>()
            .into_iter()
            .filter(|(_, FileStringQuery { path, .. })| ***path == *map_path)
            .map(|(entity, FileStringQuery { string, .. })| {
                println!("Parsing map file for entity {:?}", entity);
                let map = string.parse::<shambler::shalrath::repr::Map>().unwrap();
                let map = GeoMap::from(map);
                (entity, MapFileComponent::construct(map))
            })
            .collect::<Vec<_>>();

        for (entity, map) in components {
            world
                .insert(entity, (map,))
                .expect("Failed to add map to entity");
        }

        Ok(ctx)
    }
}
