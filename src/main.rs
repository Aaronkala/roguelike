use rltk::{Console, GameState, Point, Rltk, RGB};
use specs::prelude::*;

#[macro_use]
extern crate specs_derive;

mod components;
mod damage_system;
mod gamelog;
mod gui;
mod map;
mod map_indexing_system;
mod melee_combat_system;
mod monster_ai_system;
mod player;
mod rect;
mod visibility_system;

pub use components::*;
use damage_system::DamageSystem;
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

        self.ecs.maintain();
    }
}

impl GameState for State {
    fn tick(&mut self, ctx: &mut Rltk) {
        ctx.cls();

        let mut newrunstate;
        {
            let runstate = self.ecs.fetch::<RunState>();
            newrunstate = *runstate;
        }

        match newrunstate {
            RunState::PreRun => {
                self.run_systems();
                newrunstate = RunState::AwaitingInput;
            }
            RunState::AwaitingInput => {
                newrunstate = player::player_input(self, ctx);
            }
            RunState::PlayerTurn => {
                self.run_systems();
                newrunstate = RunState::MonsterTurn;
            }
            RunState::MonsterTurn => {
                self.run_systems();
                newrunstate = RunState::AwaitingInput;
            }
        }

        {
            let mut runwriter = self.ecs.write_resource::<RunState>();
            *runwriter = newrunstate;
        }

        damage_system::delete_the_dead(&mut self.ecs);
        map::draw_map(&self.ecs, ctx);

        let positions = self.ecs.read_storage::<Position>();
        let renderables = self.ecs.read_storage::<Renderable>();
        let map = self.ecs.fetch::<map::Map>();

        // Render all entities that are not rendered by the map, e.g.
        // player and monsters
        for (pos, render) in (&positions, &renderables).join() {
            let idx = map.xy_idx(pos.x, pos.y);
            if map.visible_tiles[idx] {
                ctx.set(pos.x, pos.y, render.fg, render.bg, render.glyph);
            }
        }

        gui::draw_ui(&self.ecs, ctx);
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

    let map = map::Map::new_map_rooms_and_corridors();
    let (player1_x, player1_y) = map.rooms[0].center();

    // Create player
    let player_entity = gs
        .ecs
        .create_entity()
        .with(Position {
            x: player1_x,
            y: player1_y,
        })
        .with(Renderable {
            glyph: rltk::to_cp437('@'),
            fg: RGB::named(rltk::YELLOW),
            bg: RGB::named(rltk::BLACK),
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
        .build();

    gs.ecs.insert(gamelog::GameLog {
        entries: vec!["Welcome to Rust Roguelike".to_string()],
    });
    // Insert player into GameState
    gs.ecs.insert(player_entity);
    // Insert player location to GameState
    gs.ecs.insert(Point::new(player1_x, player1_y));

    let mut rng = rltk::RandomNumberGenerator::new();

    // Add monsters to each room
    for (idx, room) in map.rooms.iter().skip(1).enumerate() {
        let (x, y) = room.center();
        let glyph: u8;
        let name: String;

        let roll = rng.roll_dice(1, 2);
        match roll {
            1 => {
                glyph = rltk::to_cp437('g');
                name = "Goblin".to_string();
            }
            _ => {
                glyph = rltk::to_cp437('o');
                name = "Orc".to_string();
            }
        }

        // Create monster and add to GameState
        gs.ecs
            .create_entity()
            .with(Position { x, y })
            .with(Renderable {
                glyph,
                fg: RGB::named(rltk::RED),
                bg: RGB::named(rltk::BLACK),
            })
            .with(Monster {})
            .with(CombatStats {
                max_hp: 16,
                hp: 16,
                defense: 1,
                power: 4,
            })
            .with(Name {
                name: format!("{} #{}", &name, idx),
            })
            .with(Viewshed {
                visible_tiles: Vec::new(),
                range: 8,
                dirty: true,
            })
            .with(BlocksTile {})
            .build();
    }

    // Insert Map to GameState
    gs.ecs.insert(map);
    gs.ecs.insert(RunState::PreRun);

    rltk::main_loop(context, gs);
}
