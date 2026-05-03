use rand::rngs::ThreadRng;
use rand::Rng;

use crate::dungeon::{self, Map, Tile, MAP_HEIGHT, MAP_WIDTH};
use crate::entity::{Monster, MonsterKind, Player};

#[derive(PartialEq, Eq, Clone, Copy)]
#[repr(u8)]
pub enum GameState {
    Playing = 0,
    GameOver = 1,
    Victory = 2,
    ConfirmQuit = 3,
    Disconnected = 4,
}

pub struct Game {
    pub state: GameState,
    pub map: Map,
    pub player: Player,
    pub player2: Option<Player>,
    pub monsters: Vec<Monster>,
    pub level: u8,
    pub status_msg: String,
    pub status_msg2: String,
    pub multiplayer: bool,
    pub rng: ThreadRng,
}

impl Game {
    pub fn new(multiplayer: bool) -> Self {
        let mut rng = rand::thread_rng();
        let map = dungeon::generate(&mut rng);

        let (px, py) = map.rooms.first().map(|r| r.center()).unwrap_or((1, 1));
        let player = Player::new(px, py);
        let player2 = if multiplayer {
            Some(Player::new(px + 1, py))
        } else {
            None
        };
        let monsters = spawn_monsters(&map, &mut rng, multiplayer, 1);
        let mut game_map = map;
        spawn_potions(&mut game_map, &mut rng, &monsters);
        spawn_chests(&mut game_map, &mut rng, &monsters);

        Game {
            state: GameState::Playing,
            map: game_map,
            player,
            player2,
            monsters,
            level: 1,
            status_msg: String::from("Welcome to Dagger Deep!"),
            status_msg2: String::from("Welcome to Dagger Deep!"),
            multiplayer,
            rng,
        }
    }

    pub fn next_level(&mut self) {
        self.level = self.level.saturating_add(1);
        self.map = dungeon::generate(&mut self.rng);
        let (px, py) = self.map.rooms.first().map(|r| r.center()).unwrap_or((1, 1));
        self.player.x = px;
        self.player.y = py;
        if let Some(p2) = self.player2.as_mut() {
            p2.x = px + 1;
            p2.y = py;
        }
        self.monsters = spawn_monsters(&self.map, &mut self.rng, self.multiplayer, self.level);
        spawn_potions(&mut self.map, &mut self.rng, &self.monsters);
        spawn_chests(&mut self.map, &mut self.rng, &self.monsters);
        self.status_msg = format!("Level {}. Deeper into Dagger Deep...", self.level);
        self.status_msg2 = self.status_msg.clone();
    }

    pub fn try_spawn_stair(&mut self) {
        if self.monsters.iter().any(|m| m.is_alive()) {
            return;
        }
        let mut candidates: Vec<(usize, usize)> = Vec::new();
        for y in 0..MAP_HEIGHT {
            for x in 0..MAP_WIDTH {
                if self.map.is_walkable(x, y)
                    && !(x == self.player.x && y == self.player.y)
                    && self.map.get(x, y) != Tile::Stair
                    && self.map.get(x, y) != Tile::Potion
                    && self.map.get(x, y) != Tile::Chest
                    && self.map.get(x, y) != Tile::LichChest
                {
                    candidates.push((x, y));
                }
            }
        }
        let (sx, sy) = if candidates.is_empty() {
            (self.player.x, self.player.y)
        } else {
            candidates[self.rng.gen_range(0..candidates.len())]
        };
        self.map.set(sx, sy, Tile::Stair);
        self.status_msg = String::from("The way down is revealed...");
    }

    pub fn move_player(&mut self, dx: i32, dy: i32) {
        let nx = self.player.x as i32 + dx;
        let ny = self.player.y as i32 + dy;

        if nx < 0 || ny < 0 || nx >= MAP_WIDTH as i32 || ny >= MAP_HEIGHT as i32 {
            return;
        }
        let nx = nx as usize;
        let ny = ny as usize;

        if !self.map.is_walkable(nx, ny) {
            return;
        }

        // Priority 1: monster combat
        if let Some(idx) = self.monsters.iter().position(|m| m.is_alive() && m.x == nx && m.y == ny) {
            let atk_msg = crate::combat::resolve_player_attack(
                &mut self.player,
                &mut self.monsters[idx],
                &mut self.rng,
            );
            if !self.monsters[idx].is_alive() {
                if self.monsters[idx].kind == MonsterKind::Lich {
                    self.map.set(nx, ny, Tile::LichChest);
                }
                self.status_msg = atk_msg;
                self.try_spawn_stair();
            } else if self.monsters[idx].kind == MonsterKind::Lich
                && !self.monsters[idx].teleported
                && self.monsters[idx].hp <= self.monsters[idx].max_hp / 4
            {
                self.teleport_lich(idx);
                let teleport_msg = String::from("The Lich vanishes in a flash of dark magic!");
                self.status_msg = format!("{} {}", atk_msg, teleport_msg);
                self.status_msg2 = self.status_msg.clone();
            } else {
                let (def_msg, died) = crate::combat::resolve_monster_attack(
                    &self.monsters[idx],
                    &mut self.player,
                    &mut self.rng,
                    self.multiplayer,
                );
                self.status_msg = format!("{} {}", atk_msg, def_msg);
                if died {
                    if self.multiplayer {
                        self.respawn_player1();
                        self.status_msg = String::from("You were defeated and respawned!");
                    } else {
                        self.state = GameState::GameOver;
                        return;
                    }
                }
            }
            return;
        }

        // Move player
        self.player.x = nx;
        self.player.y = ny;

        // Priority 2: potion
        if self.map.get(nx, ny) == Tile::Potion {
            self.player.hp = self.player.max_hp;
            self.map.set(nx, ny, Tile::Floor);
            self.status_msg = format!(
                "You drank a potion and feel restored! ({} HP)",
                self.player.hp
            );
            return;
        }

        // Priority 3: chest
        if self.map.get(nx, ny) == Tile::Chest {
            self.open_chest(false);
            return;
        }

        if self.map.get(nx, ny) == Tile::LichChest {
            self.open_lich_chest(false);
            return;
        }

        // Priority 4: staircase
        if self.map.get(nx, ny) == Tile::Stair {
            if self.level == 10 {
                self.state = GameState::Victory;
            } else {
                self.status_msg = String::from("The stairs lead deeper into the dungeon...");
                self.next_level();
            }
        }
    }

    fn respawn_player1(&mut self) {
        let (rx, ry) = self.map.rooms.first().map(|r| r.center()).unwrap_or((1, 1));
        self.player.x = rx;
        self.player.y = ry;
        self.player.hp = (self.player.max_hp / 2).max(1);
    }

    fn respawn_player2(&mut self) {
        let (rx, ry) = self.map.rooms.first().map(|r| r.center()).unwrap_or((1, 1));
        if let Some(p2) = self.player2.as_mut() {
            p2.x = rx + 1;
            p2.y = ry;
            p2.hp = p2.max_hp / 2;
        }
    }

    pub fn move_player2(&mut self, dx: i32, dy: i32) {
        let (cx, cy) = match &self.player2 {
            Some(p) => (p.x as i32, p.y as i32),
            None => return,
        };

        let nx = cx + dx;
        let ny = cy + dy;

        if nx < 0 || ny < 0 || nx >= MAP_WIDTH as i32 || ny >= MAP_HEIGHT as i32 {
            return;
        }
        let nx = nx as usize;
        let ny = ny as usize;

        if !self.map.is_walkable(nx, ny) {
            return;
        }

        if let Some(idx) = self.monsters.iter().position(|m| m.is_alive() && m.x == nx && m.y == ny) {
            let p2 = self.player2.as_mut().unwrap();
            let atk_msg = crate::combat::resolve_player_attack(p2, &mut self.monsters[idx], &mut self.rng);

            if !self.monsters[idx].is_alive() {
                if self.monsters[idx].kind == MonsterKind::Lich {
                    self.map.set(nx, ny, Tile::LichChest);
                }
                self.status_msg2 = atk_msg;
                self.try_spawn_stair();
            } else if self.monsters[idx].kind == MonsterKind::Lich
                && !self.monsters[idx].teleported
                && self.monsters[idx].hp <= self.monsters[idx].max_hp / 4
            {
                self.teleport_lich(idx);
                let teleport_msg = String::from("The Lich vanishes in a flash of dark magic!");
                self.status_msg2 = format!("{} {}", atk_msg, teleport_msg);
                self.status_msg = self.status_msg2.clone();
            } else {
                let (def_msg, died) = crate::combat::resolve_monster_attack(
                    &self.monsters[idx],
                    self.player2.as_mut().unwrap(),
                    &mut self.rng,
                    self.multiplayer,
                );
                self.status_msg2 = format!("{} {}", atk_msg, def_msg);
                if died {
                    self.respawn_player2();
                    self.status_msg2 = String::from("You were defeated and respawned!");
                }
            }
            return;
        }

        let p2 = self.player2.as_mut().unwrap();
        p2.x = nx;
        p2.y = ny;

        if self.map.get(nx, ny) == Tile::Potion {
            let hp = {
                let p2 = self.player2.as_mut().unwrap();
                p2.hp = p2.max_hp;
                p2.hp
            };
            self.map.set(nx, ny, Tile::Floor);
            self.status_msg2 = format!("You drank a potion and feel restored! ({} HP)", hp);
            return;
        }

        if self.map.get(nx, ny) == Tile::Chest {
            self.open_chest(true);
            return;
        }

        if self.map.get(nx, ny) == Tile::LichChest {
            self.open_lich_chest(true);
            return;
        }

        if self.map.get(nx, ny) == Tile::Stair {
            if self.level == 10 {
                self.state = GameState::Victory;
            } else {
                self.status_msg = String::from("The stairs lead deeper into the dungeon...");
                self.next_level();
            }
        }
    }

    pub fn open_chest(&mut self, is_p2: bool) {
        let (nx, ny) = if is_p2 {
            let p = self.player2.as_ref().unwrap();
            (p.x, p.y)
        } else {
            (self.player.x, self.player.y)
        };

        let roll: f32 = self.rng.r#gen();
        if roll < 0.40 {
            self.player.hp = self.player.max_hp;
            if let Some(p2) = self.player2.as_mut() {
                p2.hp = p2.max_hp;
            }
            let food_msg = if self.player2.is_some() {
                String::from("The chest contains food! Both players are fully healed!")
            } else {
                format!("The chest contains food! You are fully healed! ({} HP)", self.player.max_hp)
            };
            self.status_msg = food_msg.clone();
            self.status_msg2 = food_msg;
        } else if roll < 0.65 {
            let damage = crate::combat::roll_d6(&mut self.rng);
            if is_p2 {
                let hp_after = {
                    let p2 = self.player2.as_mut().unwrap();
                    p2.hp -= damage;
                    p2.hp
                };
                if hp_after <= 0 {
                    self.respawn_player2();
                    self.status_msg2 = String::from("The chest was a trap! You were defeated and respawned!");
                } else {
                    self.status_msg2 = format!("The chest was a trap! You take {} damage! ({} HP left)", damage, hp_after.max(0));
                }
            } else {
                self.player.hp -= damage;
                if self.player.hp <= 0 {
                    if self.multiplayer {
                        self.respawn_player1();
                        self.status_msg = String::from("The chest was a trap! You were defeated and respawned!");
                    } else {
                        self.state = GameState::GameOver;
                        self.status_msg = String::from("The chest was a trap! You have perished in Dagger Deep.");
                    }
                } else {
                    self.status_msg = format!(
                        "The chest was a trap! You take {} damage! ({} HP left)",
                        damage, self.player.hp.max(0)
                    );
                }
            }
        } else if is_p2 {
            self.status_msg2 = String::from("The chest was empty.");
        } else {
            self.status_msg = String::from("The chest was empty.");
        }
        self.map.set(nx, ny, Tile::Floor);
    }

    pub fn move_monsters(&mut self) {
        let px = self.player.x;
        let py = self.player.y;
        let p2_pos = self.player2.as_ref().map(|p| (p.x, p.y));

        for i in 0..self.monsters.len() {
            if !self.monsters[i].is_alive() {
                continue;
            }

            let mx = self.monsters[i].x as i32;
            let my = self.monsters[i].y as i32;
            let (tx, ty) = if let Some((p2x, p2y)) = p2_pos {
                let d1 = (px as i32 - mx).abs() + (py as i32 - my).abs();
                let d2 = (p2x as i32 - mx).abs() + (p2y as i32 - my).abs();
                if d2 < d1 { (p2x, p2y) } else { (px, py) }
            } else {
                (px, py)
            };
            let dx = tx as i32 - mx;
            let dy = ty as i32 - my;

            let (step_x, step_y) = if dx.abs() >= dy.abs() {
                (dx.signum(), 0)
            } else {
                (0, dy.signum())
            };

            if mx + step_x < 0 || my + step_y < 0 {
                continue;
            }
            let nx = (mx + step_x) as usize;
            let ny = (my + step_y) as usize;

            if !self.map.is_walkable(nx, ny) {
                continue;
            }
            if self.monsters.iter().any(|m| m.is_alive() && m.x == nx && m.y == ny) {
                continue;
            }

            if nx == px && ny == py {
                let (msg, died) = crate::combat::resolve_monster_attack(
                    &self.monsters[i],
                    &mut self.player,
                    &mut self.rng,
                    self.multiplayer,
                );
                self.status_msg = msg;
                if died {
                    if self.multiplayer {
                        self.respawn_player1();
                        self.status_msg = String::from("You were defeated and respawned!");
                    } else {
                        self.state = GameState::GameOver;
                        return;
                    }
                }
            } else if self.player2.as_ref().is_some_and(|p| p.x == nx && p.y == ny) {
                let (msg, died) = crate::combat::resolve_monster_attack(
                    &self.monsters[i],
                    self.player2.as_mut().unwrap(),
                    &mut self.rng,
                    self.multiplayer,
                );
                self.status_msg2 = msg;
                if died {
                    self.respawn_player2();
                    self.status_msg2 = String::from("You were defeated and respawned!");
                }
            } else {
                self.monsters[i].x = nx;
                self.monsters[i].y = ny;
            }
        }
    }

    pub fn from_net_state_solo(state: &crate::protocol::NetGameState) -> Option<Self> {
        let p2 = state.player2.as_ref()?;

        let mut map = Map::new();
        for (y, row) in state.map_tiles.iter().enumerate() {
            for (x, &tile_byte) in row.iter().enumerate() {
                map.set(x, y, Tile::from(tile_byte));
            }
        }

        let player = Player { x: p2.x, y: p2.y, hp: p2.hp, max_hp: p2.max_hp };

        let monsters = state.monsters.iter().map(|m| Monster {
            x: m.x,
            y: m.y,
            kind: MonsterKind::from(m.kind),
            hp: m.hp,
            max_hp: m.max_hp,
            alive: m.alive,
            teleported: m.teleported,
        }).collect();

        let game_state = match state.state {
            1 => GameState::GameOver,
            2 => GameState::Victory,
            _ => GameState::Playing,
        };

        Some(Game {
            state: game_state,
            map,
            player,
            player2: None,
            monsters,
            level: state.level,
            status_msg: String::from("Host disconnected. Continuing solo!"),
            status_msg2: String::new(),
            multiplayer: false,
            rng: rand::thread_rng(),
        })
    }

    fn teleport_lich(&mut self, monster_idx: usize) {
        let mut candidates: Vec<(usize, usize)> = Vec::new();
        for y in 0..MAP_HEIGHT {
            for x in 0..MAP_WIDTH {
                if self.map.is_walkable(x, y)
                    && !(x == self.player.x && y == self.player.y)
                    && self.player2.as_ref().is_none_or(|p| !(p.x == x && p.y == y))
                    && !self.monsters.iter().enumerate().any(|(i, m)| i != monster_idx && m.is_alive() && m.x == x && m.y == y)
                {
                    candidates.push((x, y));
                }
            }
        }
        if candidates.is_empty() {
            return;
        }
        let pick = self.rng.gen_range(0..candidates.len());
        let (tx, ty) = candidates[pick];
        self.monsters[monster_idx].x = tx;
        self.monsters[monster_idx].y = ty;
        self.monsters[monster_idx].teleported = true;
    }

    pub fn open_lich_chest(&mut self, is_p2: bool) {
        let (nx, ny) = if is_p2 {
            let p = self.player2.as_ref().unwrap();
            (p.x, p.y)
        } else {
            (self.player.x, self.player.y)
        };
        self.player.hp = self.player.max_hp;
        if let Some(p2) = self.player2.as_mut() {
            p2.hp = p2.max_hp;
        }
        let msg = String::from("The Lich's essence becomes a healing light! All players are fully restored!");
        self.status_msg = msg.clone();
        self.status_msg2 = msg;
        self.map.set(nx, ny, Tile::Floor);
    }

    pub fn to_net_state(&self) -> crate::protocol::NetGameState {
        use crate::protocol::{NetGameState, NetMonster, NetPlayer};
        NetGameState {
            map_tiles: self.map.tiles.iter().map(|row| {
                row.iter().map(|t| *t as u8).collect()
            }).collect(),
            player1: NetPlayer {
                x: self.player.x,
                y: self.player.y,
                hp: self.player.hp,
                max_hp: self.player.max_hp,
            },
            player2: self.player2.as_ref().map(|p| NetPlayer {
                x: p.x,
                y: p.y,
                hp: p.hp,
                max_hp: p.max_hp,
            }),
            monsters: self.monsters.iter().map(|m| NetMonster {
                x: m.x,
                y: m.y,
                kind: m.kind as u8,
                hp: m.hp,
                max_hp: m.max_hp,
                alive: m.alive,
                teleported: m.teleported,
            }).collect(),
            level: self.level,
            status_msg: self.status_msg.clone(),
            status_msg2: self.status_msg2.clone(),
            state: self.state as u8,
        }
    }
}

fn scale_for_multiplayer(monster: &mut Monster) {
    monster.hp = (monster.hp * 3 + 1) / 2;
    monster.max_hp = monster.hp;
}

fn spawn_monsters(map: &Map, rng: &mut impl Rng, multiplayer: bool, level: u8) -> Vec<Monster> {
    let rooms = &map.rooms;
    let kinds = [MonsterKind::Goblin, MonsterKind::Mage, MonsterKind::Rat];
    let mut monsters: Vec<Monster> = Vec::new();

    for (i, &kind) in kinds.iter().enumerate() {
        let room_idx = if rooms.len() > 1 {
            (i % (rooms.len() - 1)) + 1
        } else {
            0
        };
        let room = &rooms[room_idx];

        let mut placed = false;
        for _ in 0..20 {
            let mx = rng.gen_range(room.x..room.x + room.w);
            let my = rng.gen_range(room.y..room.y + room.h);
            if !monsters.iter().any(|m| m.x == mx && m.y == my) {
                let mut monster = Monster::new(mx, my, kind);
                if multiplayer { scale_for_multiplayer(&mut monster); }
                monsters.push(monster);
                placed = true;
                break;
            }
        }
        if !placed {
            'outer: for y in 0..MAP_HEIGHT {
                for x in 0..MAP_WIDTH {
                    if map.is_walkable(x, y) && !monsters.iter().any(|m| m.x == x && m.y == y) {
                        let mut monster = Monster::new(x, y, kind);
                        if multiplayer { scale_for_multiplayer(&mut monster); }
                        monsters.push(monster);
                        break 'outer;
                    }
                }
            }
        }
    }

    // Lich: 30% on level 9, 90% on level 10
    let lich_chance: f32 = match level {
        9 => 0.30,
        10 => 0.90,
        _ => 0.0,
    };
    if lich_chance > 0.0 && rng.r#gen::<f32>() < lich_chance
        && let Some(mut lich) = place_lich(map, rng, &monsters) {
            if multiplayer { scale_for_multiplayer(&mut lich); }
            monsters.push(lich);
        }

    monsters
}

fn place_lich(map: &Map, rng: &mut impl Rng, monsters: &[Monster]) -> Option<Monster> {
    // Prefer the last room (farthest from player spawn in first room)
    let rooms = &map.rooms;
    let room = rooms.last()?;
    for _ in 0..30 {
        let lx = rng.gen_range(room.x..room.x + room.w);
        let ly = rng.gen_range(room.y..room.y + room.h);
        if map.is_walkable(lx, ly) && !monsters.iter().any(|m| m.x == lx && m.y == ly) {
            return Some(Monster::new(lx, ly, MonsterKind::Lich));
        }
    }
    // Fallback: any walkable tile
    for y in 0..MAP_HEIGHT {
        for x in 0..MAP_WIDTH {
            if map.is_walkable(x, y) && !monsters.iter().any(|m| m.x == x && m.y == y) {
                return Some(Monster::new(x, y, MonsterKind::Lich));
            }
        }
    }
    None
}

fn spawn_potions(map: &mut Map, rng: &mut impl Rng, monsters: &[Monster]) {
    let count = {
        let roll: f32 = rng.r#gen();
        if roll < 0.40 { 2 } else if roll < 0.60 { 1 } else { 0 }
    };
    let mut placed = 0;
    let mut attempts = 0;
    while placed < count && attempts < 200 {
        attempts += 1;
        let x = rng.gen_range(1..MAP_WIDTH - 1);
        let y = rng.gen_range(1..MAP_HEIGHT - 1);
        if !map.is_walkable(x, y) {
            continue;
        }
        if monsters.iter().any(|m| m.x == x && m.y == y) {
            continue;
        }
        map.set(x, y, Tile::Potion);
        placed += 1;
    }
}

fn spawn_chests(map: &mut Map, rng: &mut impl Rng, monsters: &[Monster]) {
    let count = rng.gen_range(1..=2);
    let mut placed = 0;
    let mut attempts = 0;
    while placed < count && attempts < 200 {
        attempts += 1;
        let x = rng.gen_range(1..MAP_WIDTH - 1);
        let y = rng.gen_range(1..MAP_HEIGHT - 1);
        if !map.is_walkable(x, y) {
            continue;
        }
        if matches!(map.get(x, y), Tile::Potion | Tile::Stair | Tile::Chest | Tile::LichChest) {
            continue;
        }
        if monsters.iter().any(|m| m.x == x && m.y == y) {
            continue;
        }
        map.set(x, y, Tile::Chest);
        placed += 1;
    }
}
