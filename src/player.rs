extern crate rltk;
use rltk::{Point, Rltk, VirtualKeyCode};
extern crate specs;
use super::{
    gamelog::GameLog, map::Map, CombatStats, Item, Player, Position, RunState, State, Viewshed,
    WantsToMelee, WantsToPickupItem,
};
use specs::prelude::*;
use std::cmp::{max, min};

pub fn try_move_player(delta_x: i32, delta_y: i32, ecs: &mut World) {
    let mut positions = ecs.write_storage::<Position>();
    let mut players = ecs.write_storage::<Player>();
    let mut viewsheds = ecs.write_storage::<Viewshed>();
    let combat_stats = ecs.read_storage::<CombatStats>();
    let mut wants_to_melee = ecs.write_storage::<WantsToMelee>();
    let entities = ecs.entities();
    let map = ecs.fetch::<Map>();

    for (_player, pos, viewshed, entity) in
        (&mut players, &mut positions, &mut viewsheds, &entities).join()
    {
        let destination_idx = map.xy_idx(pos.x + delta_x, pos.y + delta_y);

        // There is something on the map so it's a potential target
        for potential_target in map.tile_content[destination_idx].iter() {
            // Get potential targets combat stats if it has any
            let target = combat_stats.get(*potential_target);
            // If the target has combat stats attack it
            if let Some(_t) = target {
                // Add entity to the list of mobs the player want to hit
                wants_to_melee
                    .insert(
                        entity,
                        WantsToMelee {
                            target: *potential_target,
                        },
                    )
                    .expect("Add target failed");
                return; // So we don't move after attacking
            }
        }

        // If there is nothing blocking the player movement in the destination tile
        // change player position to that tile.
        if !map.blocked[destination_idx] {
            // Don't let player walk outside map bounds
            pos.x = min(map.width - 1, max(0, pos.x + delta_x));
            pos.y = min(map.height - 1, max(0, pos.y + delta_y));
            // Mark viewshed as dirty so that it has to be recalculated.
            viewshed.dirty = true;

            // Save new player position also to GameState
            let mut ppos = ecs.write_resource::<Point>();
            ppos.x = pos.x;
            ppos.y = pos.y;
        }
    }
}

/// Controls player keyboard input
pub fn player_input(gs: &mut State, ctx: &mut Rltk) -> RunState {
    // Player movement
    match ctx.key {
        None => return RunState::AwaitingInput, // Nothing happened
        Some(key) => match key {
            // Support multiple different keyboard configurations
            VirtualKeyCode::Left | VirtualKeyCode::Numpad4 | VirtualKeyCode::H => {
                try_move_player(-1, 0, &mut gs.ecs)
            }
            VirtualKeyCode::Right | VirtualKeyCode::Numpad6 | VirtualKeyCode::L => {
                try_move_player(1, 0, &mut gs.ecs)
            }
            VirtualKeyCode::Up | VirtualKeyCode::Numpad8 | VirtualKeyCode::K => {
                try_move_player(0, -1, &mut gs.ecs)
            }
            VirtualKeyCode::Down | VirtualKeyCode::Numpad2 | VirtualKeyCode::J => {
                try_move_player(0, 1, &mut gs.ecs)
            }

            // Support diagonal movement
            VirtualKeyCode::Numpad9 | VirtualKeyCode::Y => try_move_player(1, -1, &mut gs.ecs),
            VirtualKeyCode::Numpad7 | VirtualKeyCode::U => try_move_player(-1, -1, &mut gs.ecs),
            VirtualKeyCode::Numpad3 | VirtualKeyCode::N => try_move_player(1, 1, &mut gs.ecs),
            VirtualKeyCode::Numpad1 | VirtualKeyCode::B => try_move_player(-1, 1, &mut gs.ecs),

            VirtualKeyCode::G => get_item(&mut gs.ecs),
            VirtualKeyCode::I => return RunState::ShowInventory,
            VirtualKeyCode::D => return RunState::ShowDropItem,

            // If keys don't match any of the above then don't tick game
            _ => return RunState::AwaitingInput,
        },
    }

    RunState::PlayerTurn
}

fn get_item(ecs: &mut World) {
    let player_pos = ecs.fetch::<Point>();
    let player_entity = ecs.fetch::<Entity>();
    let entities = ecs.entities();
    let items = ecs.read_storage::<Item>();
    let positions = ecs.read_storage::<Position>();
    let mut gamelog = ecs.fetch_mut::<GameLog>();

    let mut target_item: Option<Entity> = None;
    for (item_entity, _item, position) in (&entities, &items, &positions).join() {
        if position.x == player_pos.x && position.y == player_pos.y {
            target_item = Some(item_entity);
        }
    }

    match target_item {
        None => gamelog
            .entries
            .push("There is nothing here to pick up.".to_string()),
        Some(item) => {
            let mut pickup = ecs.write_storage::<WantsToPickupItem>();
            pickup
                .insert(
                    *player_entity,
                    WantsToPickupItem {
                        collected_by: *player_entity,
                        item,
                    },
                )
                .expect("Unable to insert want to pickup");
        }
    }
}
