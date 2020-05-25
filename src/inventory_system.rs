use super::{
  gamelog::GameLog, map::Map, CombatStats, Consumable, InBackpack, InflictsDamage, Name, Position,
  ProvidesHealing, SufferDamage, WantsToDropItem, WantsToPickupItem, WantsToUseItem,
};
use specs::prelude::*;

pub struct ItemCollectionSystem {}

impl<'a> System<'a> for ItemCollectionSystem {
  type SystemData = (
    ReadExpect<'a, Entity>,
    WriteExpect<'a, GameLog>,
    WriteStorage<'a, WantsToPickupItem>,
    WriteStorage<'a, Position>,
    ReadStorage<'a, Name>,
    WriteStorage<'a, InBackpack>,
  );

  fn run(&mut self, data: Self::SystemData) {
    let (player_entity, mut gamelog, mut wants_pickup, mut positions, names, mut backpack) = data;

    for pickup in wants_pickup.join() {
      positions.remove(pickup.item);
      backpack
        .insert(
          pickup.item,
          InBackpack {
            owner: pickup.collected_by,
          },
        )
        .expect("Unable to insert backpack entry");

      if pickup.collected_by == *player_entity {
        gamelog.entries.push(format!(
          "You pick up the {}.",
          names.get(pickup.item).unwrap().name
        ));
      }
    }

    wants_pickup.clear();
  }
}

pub struct ItemUseSystem {}

impl<'a> System<'a> for ItemUseSystem {
  type SystemData = (
    ReadExpect<'a, Entity>,
    WriteExpect<'a, GameLog>,
    Entities<'a>,
    WriteStorage<'a, WantsToUseItem>,
    ReadStorage<'a, Name>,
    ReadStorage<'a, ProvidesHealing>,
    WriteStorage<'a, CombatStats>,
    ReadStorage<'a, Consumable>,
    ReadStorage<'a, InflictsDamage>,
    ReadExpect<'a, Map>,
    WriteStorage<'a, SufferDamage>,
  );

  fn run(&mut self, data: Self::SystemData) {
    let (
      player_entity,
      mut gamelog,
      entities,
      mut wants_to_use,
      names,
      healing,
      mut combat_stats,
      consumables,
      inflict_damage,
      map,
      mut suffer_damage,
    ) = data;

    // Using items
    for (entity, useitem, stats) in (&entities, &wants_to_use, &mut combat_stats).join() {
      let item_heals = healing.get(useitem.item);
      match item_heals {
        None => {}
        Some(healer) => {
          stats.hp = i32::min(stats.max_hp, stats.hp + healer.heal_amount);
          if entity == *player_entity {
            gamelog.entries.push(format!(
              "You drink the {}, healing {} hp.",
              names.get(useitem.item).unwrap().name,
              healer.heal_amount
            ));
          }
        }
      }

      let item_damages = inflict_damage.get(useitem.item);
      match item_damages {
        None => {}
        Some(damager) => {
          let target_point = useitem.target.unwrap();
          let idx = map.xy_idx(target_point.x, target_point.y);

          for mob in map.tile_content[idx].iter() {
            SufferDamage::new_damage(&mut suffer_damage, *mob, damager.damage);
            if entity == *player_entity {
              let mob_name = names.get(*mob).unwrap();
              let item_name = names.get(useitem.item).unwrap();
              gamelog.entries.push(format!(
                "Did {} damage to {} with {}",
                damager.damage, mob_name.name, item_name.name
              ))
            }
          }
        }
      }

      let consumable = consumables.get(useitem.item);
      match consumable {
        None => {}
        Some(_) => {
          entities.delete(useitem.item).expect("Delete failed");
        }
      }
    }
    wants_to_use.clear();
  }
}

pub struct ItemDropSystem {}

impl<'a> System<'a> for ItemDropSystem {
  type SystemData = (
    ReadExpect<'a, Entity>,
    WriteExpect<'a, GameLog>,
    Entities<'a>,
    WriteStorage<'a, WantsToDropItem>,
    ReadStorage<'a, Name>,
    WriteStorage<'a, Position>,
    WriteStorage<'a, InBackpack>,
  );

  fn run(&mut self, data: Self::SystemData) {
    let (player_entity, mut gamelog, entities, mut wants_drop, names, mut positions, mut backpack) =
      data;

    for (entity, to_drop) in (&entities, &wants_drop).join() {
      let mut dropper_pos: Position = Position { x: 0, y: 0 };
      {
        let dropped_pos = positions.get(entity).unwrap();
        dropper_pos.x = dropped_pos.x;
        dropper_pos.y = dropped_pos.y;
      }
      positions
        .insert(
          to_drop.item,
          Position {
            x: dropper_pos.x,
            y: dropper_pos.y,
          },
        )
        .expect("Unable to insert position");
      backpack.remove(to_drop.item);

      if entity == *player_entity {
        gamelog.entries.push(format!(
          "You drop the {}.",
          names.get(to_drop.item).unwrap().name
        ));
      }
    }

    wants_drop.clear();
  }
}
