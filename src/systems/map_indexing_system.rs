extern crate specs;
use crate::{map::Map, BlocksTile, Position};
use specs::prelude::*;

pub struct MapIndexingSystem {}

impl<'a> System<'a> for MapIndexingSystem {
    type SystemData = (
        WriteExpect<'a, Map>,
        ReadStorage<'a, Position>,
        ReadStorage<'a, BlocksTile>,
        Entities<'a>,
    );

    fn run(&mut self, data: Self::SystemData) {
        let (mut map, position, blockers, entities) = data;

        // Populate blocked on map
        map.populate_blocked();
        // Clear all entities on map
        map.clear_content_index();

        // For each entity that has a position on the map
        for (position, entity) in (&position, &entities).join() {
            let idx = map.xy_idx(position.x, position.y);

            // If this entity is in the blockers list, so it has the component
            // BlocksTile
            let _p: Option<&BlocksTile> = blockers.get(entity);
            if let Some(_p) = _p {
                // Set this block as blocked
                map.blocked[idx] = true;
            }

            // Add entity to list of entities for this location in the map.
            map.tile_content[idx].push(entity);
        }
    }
}
