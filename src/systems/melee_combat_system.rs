extern crate specs;
use crate::{
    gamelog::GameLog, CombatStats, DefenseBonus, Equipped, MeleePowerBonus, Name, SufferDamage,
    WantsToMelee,
};
use specs::prelude::*;

pub struct MeleeCombatSystem {}

impl<'a> System<'a> for MeleeCombatSystem {
    type SystemData = (
        Entities<'a>,
        WriteStorage<'a, WantsToMelee>,
        ReadStorage<'a, Name>,
        ReadStorage<'a, CombatStats>,
        ReadStorage<'a, MeleePowerBonus>,
        ReadStorage<'a, DefenseBonus>,
        ReadStorage<'a, Equipped>,
        WriteStorage<'a, SufferDamage>,
        WriteExpect<'a, GameLog>,
    );

    fn run(&mut self, data: Self::SystemData) {
        let (
            entities,
            mut wants_melee,
            names,
            combat_stats,
            melee_bonuses,
            defense_bonuses,
            equipped,
            mut inflict_damage,
            mut log,
        ) = data;

        // For all entities that want to melee and have stats
        for (attacker, wants_melee, name, stats) in
            (&entities, &wants_melee, &names, &combat_stats).join()
        {
            // Only allow monsters to attack if they aren't already dead
            if stats.hp > 0 {
                let mut offensive_bonus = 0;
                for (_item_entity, equipped_item, melee_bonus) in
                    (&entities, &equipped, &melee_bonuses).join()
                {
                    if equipped_item.owner == attacker {
                        offensive_bonus += melee_bonus.power;
                    }
                }

                // Get combat stats for entity taking the damage
                let target_stats = combat_stats.get(wants_melee.target).unwrap();
                // If the entity is not dead already
                if target_stats.hp > 0 {
                    let target_name = names.get(wants_melee.target).unwrap();

                    let mut defensive_bonus = 0;
                    for (_item_entity, equipped_item, defense_bonus) in
                        (&entities, &equipped, &defense_bonuses).join()
                    {
                        if equipped_item.owner == wants_melee.target {
                            defensive_bonus += defense_bonus.defense;
                        }
                    }
                    // Calculate damage and set it as zero if less than zero
                    // Attacks shouldn't heal :)
                    let damage = i32::max(
                        0,
                        (stats.power + offensive_bonus) - (target_stats.defense + defensive_bonus),
                    );

                    if damage == 0 {
                        log.entries.push(format!(
                            "{} is unable to hurt {}",
                            &name.name, &target_name.name
                        ));
                    } else {
                        log.entries.push(format!(
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
