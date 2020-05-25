use super::{
  map, BlocksTile, CombatStats, Consumable, Entity, InflictsDamage, Item, Monster, Name, Player,
  Position, ProvidesHealing, Ranged, Rect, Renderable, Viewshed, World,
};
use rltk::{RandomNumberGenerator, RGB};
use specs::prelude::*;

const MAX_MONSTERS: i32 = 4;
const MAX_ITEMS: i32 = 6;

pub fn player(ecs: &mut World, player_x: i32, player_y: i32) -> Entity {
  ecs
    .create_entity()
    .with(Position {
      x: player_x,
      y: player_y,
    })
    .with(Renderable {
      glyph: rltk::to_cp437('@'),
      fg: RGB::named(rltk::YELLOW),
      bg: RGB::named(rltk::BLACK),
      render_order: 0,
    })
    .with(Player {})
    .with(CombatStats {
      max_hp: 30,
      hp: 30,
      defense: 2,
      power: 5,
    })
    .with(Name {
      name: "Player".to_string(),
    })
    .with(Viewshed {
      visible_tiles: Vec::new(),
      range: 8,
      dirty: true,
    })
    .build()
}

fn monster<S: ToString>(ecs: &mut World, x: i32, y: i32, glyph: u8, name: S) {
  ecs
    .create_entity()
    .with(Position { x, y })
    .with(Renderable {
      glyph,
      fg: RGB::named(rltk::RED),
      bg: RGB::named(rltk::BLACK),
      render_order: 1,
    })
    .with(Monster {})
    .with(CombatStats {
      max_hp: 16,
      hp: 16,
      defense: 1,
      power: 4,
    })
    .with(Name {
      name: name.to_string(),
    })
    .with(Viewshed {
      visible_tiles: Vec::new(),
      range: 8,
      dirty: true,
    })
    .with(BlocksTile {})
    .build();
}

fn orc(ecs: &mut World, x: i32, y: i32) {
  monster(ecs, x, y, rltk::to_cp437('o'), "Orc");
}

fn goblin(ecs: &mut World, x: i32, y: i32) {
  monster(ecs, x, y, rltk::to_cp437('g'), "Goblin");
}

pub fn random_monster(ecs: &mut World, x: i32, y: i32) {
  let roll: i32;
  {
    let mut rng = ecs.write_resource::<RandomNumberGenerator>();
    roll = rng.roll_dice(1, 2);
  }
  match roll {
    1 => orc(ecs, x, y),
    _ => goblin(ecs, x, y),
  }
}

/// Fills a room with stuff!
pub fn spawn_room(ecs: &mut World, room: &Rect) {
  let mut monster_spawn_points: Vec<usize> = Vec::new();
  let mut item_spawn_points: Vec<usize> = Vec::new();

  {
    let mut rng = ecs.write_resource::<RandomNumberGenerator>();
    let num_monsters = rng.roll_dice(1, MAX_MONSTERS + 2) - 3;
    let num_items = rng.roll_dice(1, MAX_ITEMS + 2) - 3;

    for _i in 0..num_monsters {
      let mut added = false;
      while !added {
        let x = (room.x1 + rng.roll_dice(1, i32::abs(room.x2 - room.x1))) as usize;
        let y = (room.y1 + rng.roll_dice(1, i32::abs(room.y2 - room.y1))) as usize;
        let idx = (y * map::MAPWIDTH) + x;
        if !monster_spawn_points.contains(&idx) {
          monster_spawn_points.push(idx);
          added = true;
        }
      }
    }

    for _i in 0..num_items {
      let mut added = false;
      while !added {
        let x = (room.x1 + rng.roll_dice(1, i32::abs(room.x2 - room.x1))) as usize;
        let y = (room.y1 + rng.roll_dice(1, i32::abs(room.y2 - room.y1))) as usize;
        let idx = (y * map::MAPWIDTH) + x;
        if !item_spawn_points.contains(&idx) {
          item_spawn_points.push(idx);
          added = true;
        }
      }
    }
  }

  // Spawn the monsters
  for idx in monster_spawn_points.iter() {
    let x = *idx % map::MAPWIDTH;
    let y = *idx / map::MAPWIDTH;
    random_monster(ecs, x as i32, y as i32);
  }

  // Spawn items
  for idx in item_spawn_points.iter() {
    let x = *idx % map::MAPWIDTH;
    let y = *idx / map::MAPWIDTH;
    random_item(ecs, x as i32, y as i32);
  }
}

// ---- Items ----

fn random_item(ecs: &mut World, x: i32, y: i32) {
  let roll: i32;
  {
    let mut rng = ecs.write_resource::<RandomNumberGenerator>();
    roll = rng.roll_dice(1, 2);
  }
  match roll {
    1 => health_potion(ecs, x, y),
    _ => magic_missile_scroll(ecs, x, y),
  }
}

fn health_potion(ecs: &mut World, x: i32, y: i32) {
  ecs
    .create_entity()
    .with(Position { x, y })
    .with(Renderable {
      glyph: rltk::to_cp437('¡'),
      fg: RGB::named(rltk::MAGENTA),
      bg: RGB::named(rltk::BLACK),
      render_order: 2,
    })
    .with(Name {
      name: "Health Potion".to_string(),
    })
    .with(Item {})
    .with(Consumable {})
    .with(ProvidesHealing { heal_amount: 8 })
    .build();
}

fn magic_missile_scroll(ecs: &mut World, x: i32, y: i32) {
  ecs
    .create_entity()
    .with(Position { x, y })
    .with(Renderable {
      glyph: rltk::to_cp437(')'),
      fg: RGB::named(rltk::CYAN),
      bg: RGB::named(rltk::BLACK),
      render_order: 2,
    })
    .with(Name {
      name: "Magic Missile Scroll".to_string(),
    })
    .with(Item {})
    .with(Consumable {})
    .with(Ranged { range: 6 })
    .with(InflictsDamage { damage: 8 })
    .build();
}
