use rltk::{Console, GameState, Point, Rltk};
use specs::prelude::*;

#[macro_use]
extern crate specs_derive;

mod components;
mod damage_system;
mod gamelog;
mod gui;
mod inventory_system;
mod map;
mod map_indexing_system;
mod melee_combat_system;
mod monster_ai_system;
mod player;
mod rect;
mod spawner;
mod visibility_system;

pub use components::*;
use damage_system::DamageSystem;
use inventory_system::{ItemCollectionSystem, ItemDropSystem, PotionUseSystem};
use map_indexing_system::MapIndexingSystem;
use melee_combat_system::MeleeCombatSystem;
use monster_ai_system::MonsterAI;
pub use rect::Rect;
use visibility_system::VisibilitySystem;

#[derive(PartialEq, Copy, Clone)]
pub enum RunState {
    AwaitingInput,
    PreRun,
    PlayerTurn,
    MonsterTurn,
    ShowInventory,
    ShowDropItem,
}

pub struct State {
    ecs: World,
}

impl State {
    fn run_systems(&mut self) {
        // Run Visibility System
        let mut vis = VisibilitySystem {};
        vis.run_now(&self.ecs);

        // Run Monster System
        let mut mob = MonsterAI {};
        mob.run_now(&self.ecs);

        // Run Map Indexing System
        let mut mapindex = MapIndexingSystem {};
        mapindex.run_now(&self.ecs);

        // Run Melee Combat System
        let mut melee = MeleeCombatSystem {};
        melee.run_now(&self.ecs);

        // Run Damage System
        let mut damage = DamageSystem {};
        damage.run_now(&self.ecs);

        // Run Item Collection System
        let mut pickup = ItemCollectionSystem {};
        pickup.run_now(&self.ecs);

        let mut potions = PotionUseSystem {};
        potions.run_now(&self.ecs);

        let mut drop_items = ItemDropSystem {};
        drop_items.run_now(&self.ecs);

        self.ecs.maintain();
    }
}

impl GameState for State {
    fn tick(&mut self, ctx: &mut Rltk) {
        ctx.cls();

        map::draw_map(&self.ecs, ctx);

        {
            let positions = self.ecs.read_storage::<Position>();
            let renderables = self.ecs.read_storage::<Renderable>();
            let map = self.ecs.fetch::<map::Map>();

            let mut data = (&positions, &renderables).join().collect::<Vec<_>>();
            data.sort_by(|&a, &b| b.1.render_order.cmp(&a.1.render_order));
            for (pos, render) in data.iter() {
                let idx = map.xy_idx(pos.x, pos.y);
                if map.visible_tiles[idx] {
                    ctx.set(pos.x, pos.y, render.fg, render.bg, render.glyph)
                }
            }
            gui::draw_ui(&self.ecs, ctx);
        }

        let mut newrunstate;
        {
            let runstate = self.ecs.fetch::<RunState>();
            newrunstate = *runstate;
        }

        match newrunstate {
            RunState::PreRun => {
                self.run_systems();
                self.ecs.maintain();
                newrunstate = RunState::AwaitingInput;
            }
            RunState::AwaitingInput => {
                newrunstate = player::player_input(self, ctx);
            }
            RunState::PlayerTurn => {
                self.run_systems();
                self.ecs.maintain();
                newrunstate = RunState::MonsterTurn;
            }
            RunState::MonsterTurn => {
                self.run_systems();
                self.ecs.maintain();
                newrunstate = RunState::AwaitingInput;
            }
            RunState::ShowInventory => {
                let result = gui::show_inventory(self, ctx);
                match result.0 {
                    gui::ItemMenuResult::Cancel => newrunstate = RunState::AwaitingInput,
                    gui::ItemMenuResult::NoResponse => {}
                    gui::ItemMenuResult::Selected => {
                        let item_entity = result.1.unwrap();
                        let mut intent = self.ecs.write_storage::<WantsToDrinkPotion>();
                        intent
                            .insert(
                                *self.ecs.fetch::<Entity>(),
                                WantsToDrinkPotion {
                                    potion: item_entity,
                                },
                            )
                            .expect("Unable to insert intent");
                        newrunstate = RunState::PlayerTurn;
                    }
                }
            }
            RunState::ShowDropItem => {
                let result = gui::drop_item_menu(self, ctx);
                match result.0 {
                    gui::ItemMenuResult::Cancel => newrunstate = RunState::AwaitingInput,
                    gui::ItemMenuResult::NoResponse => {}
                    gui::ItemMenuResult::Selected => {
                        let item_entity = result.1.unwrap();
                        let mut intent = self.ecs.write_storage::<WantsToDropItem>();
                        intent
                            .insert(
                                *self.ecs.fetch::<Entity>(),
                                WantsToDropItem { item: item_entity },
                            )
                            .expect("Unable to insert intent");
                        newrunstate = RunState::PlayerTurn;
                    }
                }
            }
        }

        {
            let mut runwriter = self.ecs.write_resource::<RunState>();
            *runwriter = newrunstate;
        }

        damage_system::delete_the_dead(&mut self.ecs);
    }
}

fn main() {
    use rltk::RltkBuilder;
    let mut context = RltkBuilder::simple80x50()
        .with_title("Roguelike Tutorial")
        .build();
    context.with_post_scanlines(true);

    let mut gs = State { ecs: World::new() };

    // Register all components
    gs.ecs.register::<Position>();
    gs.ecs.register::<Renderable>();
    gs.ecs.register::<Player>();
    gs.ecs.register::<Viewshed>();
    gs.ecs.register::<Monster>();
    gs.ecs.register::<Name>();
    gs.ecs.register::<BlocksTile>();
    gs.ecs.register::<CombatStats>();
    gs.ecs.register::<WantsToMelee>();
    gs.ecs.register::<SufferDamage>();
    gs.ecs.register::<Item>();
    gs.ecs.register::<Potion>();
    gs.ecs.register::<InBackpack>();
    gs.ecs.register::<WantsToPickupItem>();
    gs.ecs.register::<WantsToDrinkPotion>();
    gs.ecs.register::<WantsToDropItem>();
    gs.ecs.insert(rltk::RandomNumberGenerator::new());

    let map = map::Map::new_map_rooms_and_corridors();
    let (player1_x, player1_y) = map.rooms[0].center();
    // Create player
    let player_entity = spawner::player(&mut gs.ecs, player1_x, player1_y);
    // Insert player into GameState
    gs.ecs.insert(player_entity);
    // Insert player location to GameState
    gs.ecs.insert(Point::new(player1_x, player1_y));

    // Add monsters to each room
    for room in map.rooms.iter().skip(1) {
        spawner::spawn_room(&mut gs.ecs, room);
    }

    // Insert Map to GameState
    gs.ecs.insert(map);
    gs.ecs.insert(RunState::PreRun);
    gs.ecs.insert(gamelog::GameLog {
        entries: vec!["Welcome to Rust Roguelike".to_string()],
    });

    rltk::main_loop(context, gs);
}
