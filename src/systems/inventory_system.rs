use crate::{
  gamelog::GameLog, map::Map, particle_system::ParticleBuilder, AreaOfEffect, CombatStats,
  Confusion, Consumable, Equippable, Equipped, InBackpack, InflictsDamage, Name, Position,
  ProvidesHealing, SufferDamage, WantsToDropItem, WantsToPickupItem, WantsToUnequipItem,
  WantsToUseItem,
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
    ReadStorage<'a, AreaOfEffect>,
    ReadExpect<'a, Map>,
    WriteStorage<'a, SufferDamage>,
    WriteStorage<'a, Confusion>,
    WriteStorage<'a, Equippable>,
    WriteStorage<'a, Equipped>,
    WriteStorage<'a, InBackpack>,
    WriteExpect<'a, ParticleBuilder>,
    ReadStorage<'a, Position>,
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
      aoe,
      map,
      mut suffer_damage,
      mut confused,
      equippable,
      mut equipped,
      mut backpack,
      mut particle_builder,
      positions,
    ) = data;

    // Using items
    for (entity, useitem) in (&entities, &wants_to_use).join() {
      // Targeting
      let mut targets: Vec<Entity> = Vec::new();
      match useitem.target {
        None => {
          targets.push(*player_entity);
        }
        Some(target) => {
          let area_effect = aoe.get(useitem.item);
          match area_effect {
            None => {
              // Single point target
              let idx = map.xy_idx(target.x, target.y);
              for mob in map.tile_content[idx].iter() {
                targets.push(*mob);
              }
            }
            Some(area_effect) => {
              // AOE target
              let mut blast_tiles = rltk::field_of_view(target, area_effect.radius, &*map);
              blast_tiles
                .retain(|p| p.x > 0 && p.x < map.width - 1 && p.y > 0 && p.y < map.height - 1);
              for tile_idx in blast_tiles.iter() {
                let idx = map.xy_idx(tile_idx.x, tile_idx.y);
                for mob in map.tile_content[idx].iter() {
                  targets.push(*mob);
                }
                particle_builder.request(
                  tile_idx.x,
                  tile_idx.y,
                  rltk::RGB::named(rltk::ORANGE),
                  rltk::RGB::named(rltk::BLACK),
                  rltk::to_cp437('░'),
                  200.0,
                );
              }
            }
          }
        }
      }

      let item_equippable = equippable.get(useitem.item);
      match item_equippable {
        None => {}
        Some(equipment) => {
          let target = targets[0];
          // Contains a list of entities that the target has equipped and that
          // should be unequipped
          let mut to_unequip: Vec<Entity> = Vec::new();
          // Clear equipment slot
          for (item_entity, already_equipped, name) in (&entities, &equipped, &names).join() {
            // If an equipment is already equipped in the same slot on the target
            if already_equipped.owner == target && already_equipped.slot == equipment.slot {
              to_unequip.push(item_entity);
              if target == *player_entity {
                gamelog.entries.push(format!("You unequip {}.", name.name))
              }
            }
          }
          for item in to_unequip.iter() {
            equipped.remove(*item);
            backpack
              .insert(*item, { InBackpack { owner: target } })
              .expect("Item could not be inserted to the backpack");
          }

          // Equip item in slot
          equipped
            .insert(
              useitem.item,
              Equipped {
                owner: target,
                slot: equipment.slot,
              },
            )
            .expect("Item could not be equipped");
          backpack.remove(useitem.item);
          if target == *player_entity {
            let name = names.get(useitem.item);
            if let Some(name) = name {
              gamelog.entries.push(format!("You equipped {}", name.name))
            }
          }
        }
      }

      let item_heals = healing.get(useitem.item);
      match item_heals {
        None => {}
        Some(healer) => {
          for target in targets.iter() {
            let stats = combat_stats.get_mut(*target);

            if let Some(stats) = stats {
              stats.hp = i32::min(stats.max_hp, stats.hp + healer.heal_amount);
              if entity == *player_entity {
                gamelog.entries.push(format!(
                  "You drink the {}, healing {} hp.",
                  names.get(useitem.item).unwrap().name,
                  healer.heal_amount
                ));
              }
              let pos = positions.get(*target);
              if let Some(pos) = pos {
                particle_builder.request(
                  pos.x,
                  pos.y,
                  rltk::RGB::named(rltk::GREEN),
                  rltk::RGB::named(rltk::BLACK),
                  rltk::to_cp437('♥'),
                  200.0,
                );
              }
            }
          }
        }
      }

      let item_damages = inflict_damage.get(useitem.item);
      match item_damages {
        None => {}
        Some(damager) => {
          for target in targets.iter() {
            SufferDamage::new_damage(&mut suffer_damage, *target, damager.damage);

            if entity == *player_entity {
              let mob_name = names.get(*target).unwrap();
              let item_name = names.get(useitem.item).unwrap();
              gamelog.entries.push(format!(
                "Did {} damage to {} with {}",
                damager.damage, mob_name.name, item_name.name
              ))
            }

            let pos = positions.get(*target);
            if let Some(pos) = pos {
              particle_builder.request(
                pos.x,
                pos.y,
                rltk::RGB::named(rltk::RED),
                rltk::RGB::named(rltk::BLACK),
                rltk::to_cp437('‼'),
                200.0,
              );
            }
          }
        }
      }

      // Can it pass along confusion? Note the use of scopes to escape from the borrow checker!
      let mut add_confusion = Vec::new();
      {
        let causes_confusion = confused.get(useitem.item);
        match causes_confusion {
          None => {}
          Some(confusion) => {
            for mob in targets.iter() {
              add_confusion.push((*mob, confusion.turns));
              if entity == *player_entity {
                let mob_name = names.get(*mob).unwrap();
                let item_name = names.get(useitem.item).unwrap();
                gamelog.entries.push(format!(
                  "You use {} on {}, confusing them.",
                  item_name.name, mob_name.name
                ));
              }

              let pos = positions.get(*mob);
              if let Some(pos) = pos {
                particle_builder.request(
                  pos.x,
                  pos.y,
                  rltk::RGB::named(rltk::MAGENTA),
                  rltk::RGB::named(rltk::BLACK),
                  rltk::to_cp437('?'),
                  200.0,
                );
              }
            }
          }
        }
      }
      for mob in add_confusion.iter() {
        confused
          .insert(mob.0, Confusion { turns: mob.1 })
          .expect("Unable to insert status");
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

pub struct ItemUnequipSystem {}

impl<'a> System<'a> for ItemUnequipSystem {
  type SystemData = (
    Entities<'a>,
    WriteStorage<'a, WantsToUnequipItem>,
    WriteStorage<'a, InBackpack>,
    WriteStorage<'a, Equipped>,
  );

  fn run(&mut self, data: Self::SystemData) {
    let (entities, mut wants_to_unequip, mut in_backpack, mut equipped) = data;

    for (entity, item_to_unequip) in (&entities, &wants_to_unequip).join() {
      equipped.remove(item_to_unequip.item);
      in_backpack
        .insert(item_to_unequip.item, InBackpack { owner: entity })
        .expect("Failed to insert item to backpack");
    }

    wants_to_unequip.clear();
  }
}
