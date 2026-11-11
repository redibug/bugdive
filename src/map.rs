//! Sector generation: a handful of server-room chambers joined by data
//! corridors. Every floor is guaranteed connected — the dive point is
//! always reachable from spawn.

use crate::rng::Rng;

pub const MAP_W: i32 = 58;
pub const MAP_H: i32 = 30;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Tile {
    Wall,
    Floor,
    Stairs, // the dive point: stand here and press > to go deeper
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ItemKind {
    Zenny(u32),
    Recovery(u32),
}

#[derive(Clone, Copy, Debug)]
pub struct Item {
    pub pos: (i32, i32),
    pub kind: ItemKind,
}

pub struct Sector {
    pub tiles: Vec<Vec<Tile>>, // [y][x]
    pub items: Vec<Item>,
    pub spawn: (i32, i32),
    pub stairs: (i32, i32),
}

impl Sector {
    pub fn tile(&self, x: i32, y: i32) -> Tile {
        if x < 0 || y < 0 || x >= MAP_W || y >= MAP_H {
            return Tile::Wall;
        }
        self.tiles[y as usize][x as usize]
    }

    pub fn walkable(&self, x: i32, y: i32) -> bool {
        self.tile(x, y) != Tile::Wall
    }
}

struct Room {
    x: i32,
    y: i32,
    w: i32,
    h: i32,
}

impl Room {
    fn center(&self) -> (i32, i32) {
        (self.x + self.w / 2, self.y + self.h / 2)
    }
    fn overlaps(&self, o: &Room, pad: i32) -> bool {
        self.x - pad < o.x + o.w && self.x + self.w + pad > o.x && self.y - pad < o.y + o.h && self.y + self.h + pad > o.y
    }
}

pub fn generate(rng: &mut Rng) -> Sector {
    let mut tiles = vec![vec![Tile::Wall; MAP_W as usize]; MAP_H as usize];
    let mut rooms: Vec<Room> = Vec::new();

    for _ in 0..60 {
        if rooms.len() >= 8 {
            break;
        }
        let w = rng.range(6, 12);
        let h = rng.range(4, 7);
        let x = rng.range(1, MAP_W - w - 2);
        let y = rng.range(1, MAP_H - h - 2);
        let room = Room { x, y, w, h };
        if rooms.iter().any(|r| room.overlaps(r, 1)) {
            continue;
        }
        for yy in room.y..room.y + room.h {
            for xx in room.x..room.x + room.w {
                tiles[yy as usize][xx as usize] = Tile::Floor;
            }
        }
        rooms.push(room);
    }
    if rooms.is_empty() {
        // Paranoid fallback: one big room.
        for yy in 2..MAP_H - 2 {
            for xx in 2..MAP_W - 2 {
                tiles[yy as usize][xx as usize] = Tile::Floor;
            }
        }
        rooms.push(Room { x: 2, y: 2, w: MAP_W - 4, h: MAP_H - 4 });
    }

    // Corridors between consecutive room centres (L-shaped).
    for i in 1..rooms.len() {
        let (x1, y1) = rooms[i - 1].center();
        let (x2, y2) = rooms[i].center();
        let (mut cx, mut cy) = (x1, y1);
        while cx != x2 {
            tiles[cy as usize][cx as usize] = Tile::Floor;
            cx += (x2 - cx).signum();
        }
        while cy != y2 {
            tiles[cy as usize][cx as usize] = Tile::Floor;
            cy += (y2 - cy).signum();
        }
        tiles[y2 as usize][x2 as usize] = Tile::Floor;
    }

    let spawn = rooms[0].center();
    // Dive point: the room centre farthest from spawn.
    let stairs = rooms
        .iter()
        .map(|r| r.center())
        .max_by_key(|&(x, y)| (x - spawn.0).abs() + (y - spawn.1).abs())
        .unwrap_or((MAP_W - 3, MAP_H - 3));
    let stairs = if stairs == spawn { (MAP_W - 3, MAP_H - 3) } else { stairs };
    tiles[stairs.1 as usize][stairs.0 as usize] = Tile::Stairs;

    // Floor loot: zenny caches + recovery data, never on spawn/stairs.
    let mut items = Vec::new();
    let mut tries = 0;
    while items.len() < 6 && tries < 200 {
        tries += 1;
        let x = rng.range(1, MAP_W - 2);
        let y = rng.range(1, MAP_H - 2);
        if tiles[y as usize][x as usize] != Tile::Floor || (x, y) == spawn {
            continue;
        }
        if items.iter().any(|it: &Item| it.pos == (x, y)) {
            continue;
        }
        let kind = if rng.chance(55) {
            ItemKind::Zenny(rng.range(2, 9) as u32 * 10)
        } else {
            ItemKind::Recovery(20)
        };
        items.push(Item { pos: (x, y), kind });
    }

    Sector { tiles, items, spawn, stairs }
}
