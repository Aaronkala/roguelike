extern crate rltk;
use super::Rect;
use rltk::{Algorithm2D, BaseMap, Console, Point, RandomNumberGenerator, Rltk, RGB};
use std::cmp::{max, min};
extern crate specs;
use specs::prelude::*;

#[derive(PartialEq, Copy, Clone)]
pub enum TileType {
    Wall,
    Floor,
}

pub const MAPWIDTH: usize = 80;
pub const MAPHEIGHT: usize = 43;
pub const MAPCOUNT: usize = MAPHEIGHT * MAPWIDTH;

pub struct Map {
    pub tiles: Vec<TileType>,
    pub rooms: Vec<Rect>,
    pub width: i32,
    pub height: i32,
    pub revealed_tiles: Vec<bool>,
    pub visible_tiles: Vec<bool>,
    pub blocked: Vec<bool>,
    pub tile_content: Vec<Vec<Entity>>,
}

impl Map {
    /// Get x and y coordinate as index of array
    pub fn xy_idx(&self, x: i32, y: i32) -> usize {
        (y as usize * self.width as usize) + x as usize
    }

    /// Adds a the room provided to the map
    fn apply_room_to_map(&mut self, room: &Rect) {
        for y in room.y1 + 1..=room.y2 {
            for x in room.x1 + 1..=room.x2 {
                let idx = self.xy_idx(x, y);
                self.tiles[idx] = TileType::Floor;
            }
        }
    }

    /// Add a horizontal tunnel between two rooms
    fn apply_horizontal_tunnel(&mut self, x1: i32, x2: i32, y: i32) {
        for x in min(x1, x2)..=max(x1, x2) {
            let idx = self.xy_idx(x, y);
            if idx > 0 && idx < 80 * 50 {
                self.tiles[idx as usize] = TileType::Floor;
            }
        }
    }

    /// Add a vertical tunnel between two rooms
    fn apply_vertical_tunnel(&mut self, y1: i32, y2: i32, x: i32) {
        for y in min(y1, y2)..=max(y1, y2) {
            let idx = self.xy_idx(x, y);
            if idx > 0 && idx < 80 * 50 {
                self.tiles[idx as usize] = TileType::Floor;
            }
        }
    }

    /// Generate a new map with rooms and corridors
    pub fn new_map_rooms_and_corridors() -> Map {
        let mut map = Map {
            tiles: vec![TileType::Wall; MAPWIDTH * MAPHEIGHT],
            rooms: Vec::new(),
            width: MAPWIDTH as i32,
            height: MAPHEIGHT as i32,
            revealed_tiles: vec![false; MAPWIDTH * MAPHEIGHT],
            visible_tiles: vec![false; MAPWIDTH * MAPHEIGHT],
            blocked: vec![false; MAPWIDTH * MAPHEIGHT],
            tile_content: vec![Vec::new(); MAPWIDTH * MAPHEIGHT],
        };

        const MAX_ROOMS: i32 = 30;
        const MIN_SIZE: i32 = 6;
        const MAX_SIZE: i32 = 10;

        let mut rng = RandomNumberGenerator::new();
        for _ in 0..MAX_ROOMS {
            let w = rng.range(MIN_SIZE, MAX_SIZE);
            let h = rng.range(MIN_SIZE, MAX_SIZE);
            let x = rng.roll_dice(1, map.width - w - 2);
            let y = rng.roll_dice(1, map.height - h - 2);

            let new_room = Rect::new(x, y, w, h);

            let mut ok = true;
            for other_room in map.rooms.iter() {
                if new_room.intersect(other_room) {
                    ok = false
                }
            }

            if ok {
                map.apply_room_to_map(&new_room);
                if !map.rooms.is_empty() {
                    let (new_x, new_y) = new_room.center();
                    let (prev_x, prev_y) = map.rooms[map.rooms.len() - 1].center();
                    if rng.range(0, 2) == 1 {
                        map.apply_horizontal_tunnel(prev_x, new_x, prev_y);
                        map.apply_vertical_tunnel(prev_y, new_y, new_x);
                    } else {
                        map.apply_horizontal_tunnel(prev_x, new_x, new_y);
                        map.apply_vertical_tunnel(prev_y, new_y, prev_x);
                    };
                }
                map.rooms.push(new_room);
            }
        }

        map
    }

    /// Check if point on map is a valid location to move to
    fn is_exit_valid(&self, x: i32, y: i32) -> bool {
        if x < 1 || x > self.width - 1 || y < 1 || y > self.height - 1 {
            return false;
        }
        let idx = self.xy_idx(x, y);
        !self.blocked[idx]
    }

    /// Sets walls as blockers
    pub fn populate_blocked(&mut self) {
        for (i, tile) in self.tiles.iter_mut().enumerate() {
            self.blocked[i] = *tile == TileType::Wall;
        }
    }

    /// Clears all entities from map
    pub fn clear_content_index(&mut self) {
        for content in self.tile_content.iter_mut() {
            content.clear();
        }
    }
}

impl BaseMap for Map {
    fn is_opaque(&self, idx: usize) -> bool {
        self.tiles[idx] == TileType::Wall
    }

    /// Used in a_star path finding algorithm to find the distance from
    /// point 1 to point 2
    fn get_pathing_distance(&self, idx1: usize, idx2: usize) -> f32 {
        let w = self.width as usize;
        let p1 = Point::new(idx1 % w, idx1 / w);
        let p2 = Point::new(idx2 % w, idx2 / w);
        rltk::DistanceAlg::Pythagoras.distance2d(p1, p2)
    }

    /// Used in a_star path finding algorithm to find the available blocks
    /// around a certain position, for example the location of a monster.
    fn get_available_exits(&self, idx: usize) -> Vec<(usize, f32)> {
        let mut exits: Vec<(usize, f32)> = Vec::new();
        let x = idx as i32 % self.width;
        let y = idx as i32 / self.width;
        let w = self.width as usize;

        // Cardinal directions
        if self.is_exit_valid(x - 1, y) {
            exits.push((idx - 1, 1.0))
        };
        if self.is_exit_valid(x + 1, y) {
            exits.push((idx + 1, 1.0))
        };
        if self.is_exit_valid(x, y - 1) {
            exits.push((idx - w, 1.0))
        };
        if self.is_exit_valid(x, y + 1) {
            exits.push((idx + w, 1.0))
        };

        // Diagonals
        if self.is_exit_valid(x - 1, y - 1) {
            exits.push(((idx - w) - 1, 1.45));
        }
        if self.is_exit_valid(x + 1, y - 1) {
            exits.push(((idx - w) + 1, 1.45));
        }
        if self.is_exit_valid(x - 1, y + 1) {
            exits.push(((idx + w) - 1, 1.45));
        }
        if self.is_exit_valid(x + 1, y + 1) {
            exits.push(((idx + w) + 1, 1.45));
        }

        exits
    }
}

impl Algorithm2D for Map {
    fn dimensions(&self) -> Point {
        Point::new(self.width, self.height)
    }
}

/// Draws map to the screen. It contains: floors and walls
pub fn draw_map(ecs: &World, ctx: &mut Rltk) {
    let map = ecs.fetch::<Map>();

    let mut y = 0;
    let mut x = 0;

    // Loop through the whole map
    for (idx, tile) in map.tiles.iter().enumerate() {
        // Paint only the tiles the player has visited to the screen
        if map.revealed_tiles[idx] {
            let glyph;
            let mut fg;

            // Render a tile depending upon the tile type
            match tile {
                TileType::Floor => {
                    glyph = rltk::to_cp437('.');
                    fg = RGB::from_f32(0.0, 0.4, 0.4);
                }
                TileType::Wall => {
                    glyph = rltk::to_cp437('#');
                    fg = RGB::from_f32(0.0, 0.7, 0.4);
                }
            }

            // Darkens the blocks that are not close to the player
            // Creates the effect that the player is carrying a torch
            if !map.visible_tiles[idx] {
                fg = fg.lerp(RGB::from_f32(0.0, 0.0, 0.0), 0.8);
            }

            ctx.set(x, y, fg, RGB::from_f32(0., 0., 0.), glyph);
        }

        // Go to next coor on the right or to the next row
        x += 1;
        if x > map.width - 1 {
            x = 0;
            y += 1;
        }
    }
}
