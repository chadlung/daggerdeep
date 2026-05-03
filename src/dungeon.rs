use rand::Rng;

pub const MAP_WIDTH: usize = 81;
pub const MAP_HEIGHT: usize = 28;

// Compile-time assertions to prevent usize underflow in room placement
const _: () = assert!(10 + 1 < MAP_WIDTH, "max room width too large for map");
const _: () = assert!(8 + 1 < MAP_HEIGHT, "max room height too large for map");

#[derive(Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum Tile {
    Wall = 0,
    Floor = 1,
    Corridor = 2,
    Stair = 3,
    Potion = 4,
    Chest = 5,
    LichChest = 6,
}

impl From<u8> for Tile {
    fn from(b: u8) -> Self {
        match b {
            1 => Tile::Floor,
            2 => Tile::Corridor,
            3 => Tile::Stair,
            4 => Tile::Potion,
            5 => Tile::Chest,
            6 => Tile::LichChest,
            _ => Tile::Wall,
        }
    }
}

#[derive(Clone, Copy)]
pub struct Room {
    pub x: usize,
    pub y: usize,
    pub w: usize,
    pub h: usize,
}

impl Room {
    pub fn new(x: usize, y: usize, w: usize, h: usize) -> Self {
        Room { x, y, w, h }
    }

    pub fn center(&self) -> (usize, usize) {
        (self.x + self.w / 2, self.y + self.h / 2)
    }

    pub fn overlaps(&self, other: &Room) -> bool {
        self.x < other.x.saturating_add(other.w).saturating_add(1)
            && self.x.saturating_add(self.w).saturating_add(1) > other.x
            && self.y < other.y.saturating_add(other.h).saturating_add(1)
            && self.y.saturating_add(self.h).saturating_add(1) > other.y
    }
}

pub struct Map {
    pub tiles: Vec<Vec<Tile>>,   // tiles[y][x]
    pub rooms: Vec<Room>,
}

impl Map {
    pub fn new() -> Self {
        Map {
            tiles: vec![vec![Tile::Wall; MAP_WIDTH]; MAP_HEIGHT],
            rooms: Vec::new(),
        }
    }

    pub fn get(&self, x: usize, y: usize) -> Tile {
        if x >= MAP_WIDTH || y >= MAP_HEIGHT {
            return Tile::Wall;
        }
        self.tiles[y][x]
    }

    pub fn set(&mut self, x: usize, y: usize, tile: Tile) {
        if x >= MAP_WIDTH || y >= MAP_HEIGHT {
            return;
        }
        self.tiles[y][x] = tile;
    }

    pub fn is_walkable(&self, x: usize, y: usize) -> bool {
        if x >= MAP_WIDTH || y >= MAP_HEIGHT {
            return false;
        }
        matches!(
            self.tiles[y][x],
            Tile::Floor | Tile::Corridor | Tile::Stair | Tile::Potion | Tile::Chest | Tile::LichChest
        )
    }
}

pub fn generate(rng: &mut impl Rng) -> Map {
    // Retry up to 10 times to get a map with at least 4 rooms
    for _ in 0..10 {
        let map = try_generate(rng);
        if map.rooms.len() >= 4 {
            return map;
        }
    }
    // Final attempt — accept whatever we get (should not happen in practice)
    try_generate(rng)
}

fn try_generate(rng: &mut impl Rng) -> Map {
    let mut map = Map::new();
    let num_rooms = rng.gen_range(6..=10);

    for _ in 0..50 {
        if map.rooms.len() >= num_rooms {
            break;
        }
        let w = rng.gen_range(4..=10);
        let h = rng.gen_range(4..=8);
        let x = rng.gen_range(1..MAP_WIDTH - w - 1);
        let y = rng.gen_range(1..MAP_HEIGHT - h - 1);
        let room = Room::new(x, y, w, h);

        if map.rooms.iter().any(|r| r.overlaps(&room)) {
            continue;
        }

        for ry in room.y..room.y + room.h {
            for rx in room.x..room.x + room.w {
                map.set(rx, ry, Tile::Floor);
            }
        }

        if let Some(prev) = map.rooms.last() {
            let (px, py) = prev.center();
            let (cx, cy) = room.center();
            carve_h_corridor(&mut map, px, cx, py);
            carve_v_corridor(&mut map, py, cy, cx);
        }

        map.rooms.push(room);
    }

    map
}

fn carve_h_corridor(map: &mut Map, x1: usize, x2: usize, y: usize) {
    let (start, end) = if x1 < x2 { (x1, x2) } else { (x2, x1) };
    for x in start..=end {
        if map.get(x, y) == Tile::Wall {
            map.set(x, y, Tile::Corridor);
        }
    }
}

fn carve_v_corridor(map: &mut Map, y1: usize, y2: usize, x: usize) {
    let (start, end) = if y1 < y2 { (y1, y2) } else { (y2, y1) };
    for y in start..=end {
        if map.get(x, y) == Tile::Wall {
            map.set(x, y, Tile::Corridor);
        }
    }
}
