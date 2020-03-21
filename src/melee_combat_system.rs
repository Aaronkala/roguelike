extern crate specs;
use super::{CombatStats, Name, SufferDamage, WantsToMelee};
use rltk::console;
use specs::prelude::*;

pub struct MeleeCombatSystem {}

impl<'a> System<'a> for MeleeCombatSystem {
    type SystemData = (
        Entities<'a>,
        WriteStorage<'a, WantsToMelee>,
        ReadStorage<'a, Name>,
        ReadStorage<'a, CombatStats>,
        WriteStorage<'a, SufferDamage>,
    );

    fn run(&mut self, data: Self::SystemData) {
        let (entities, mut wants_melee, names, combat_stats, mut inflict_damage) = data;

        // For all entities that want to melee and have stats
        for (_entity, wants_melee, name, stats) in
            (&entities, &wants_melee, &names, &combat_stats).join()
        {
            // Only allow monsters to attack if they aren't already dead
            if stats.hp > 0 {
                // Get combat stats for entity taking the damage
                let target_stats = combat_stats.get(wants_melee.target).unwrap();

                // If the entity is not dead already
                if target_stats.hp > 0 {
                    let target_name = names.get(wants_melee.target).unwrap();

                    // Calculate damage and set it as zero if less than zero
                    // Attacks shouldn't heal :)
                    let damage = i32::max(0, stats.power - target_stats.defense);

                    if damage == 0 {
                        console::log(&format!(
                            "{} is unable to hurt {}",
                            &name.name, &target_name.name
                        ));
                    } else {
                        console::log(&format!(
                            "{} hits {}, for {} hp.",
                            &name.name, &target_name.name, damage
                        ));
                        SufferDamage::new_damage(&mut inflict_damage, wants_melee.target, damage);
                    }
                }
            }
        }

        // After all attacks remove them from the list
        wants_melee.clear();
    }
}
