# Return to Dagger Deep — Multiplayer Co-op Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add two-player cooperative TCP multiplayer, chest mechanic, and difficulty scaling while leaving single-player mode completely unchanged.

**Architecture:** Threaded sync TCP — host owns all `Game` state and sends full `NetGameState` snapshots after each turn; client is a dumb display terminal. `std::sync::mpsc` channels bridge recv threads to the main game loop. Three new modules: `protocol`, `net`, `menu`.

**Tech Stack:** Rust (edition 2024), Ratatui 0.29, Crossterm 0.28, Rand 0.8, Serde 1 + Bincode 1 (new)

**Note:** No tests are required for this project. No git commits are required.

---

## File Map

| File | Change |
|------|--------|
| `Cargo.toml` | Add `serde`, `bincode` |
| `src/protocol.rs` | **New** — `ClientMsg`, `ServerMsg`, `NetGameState`, `NetPlayer`, `NetMonster` |
| `src/net.rs` | **New** — `send_msg` / `recv_msg` with u32 length-prefix framing |
| `src/menu.rs` | **New** — TUI start screen with single-player / host / join flows |
| `src/dungeon.rs` | Add `#[repr(u8)]` + explicit values to `Tile`; add `Tile::Chest`; update `is_walkable`; add `From<u8> for Tile` |
| `src/entity.rs` | Add `#[repr(u8)]` + explicit values to `MonsterKind`; add `From<u8> for MonsterKind` |
| `src/combat.rs` | `resolve_monster_attack` gains `multiplayer: bool`; rolls with advantage when true |
| `src/game.rs` | Add `#[repr(u8)]` to `GameState`; add `Disconnected` variant; add `player2`, `multiplayer` to `Game`; update `new`, `next_level`, `spawn_monsters`, `move_player`, `move_monsters`; add `move_player2`, `open_chest`, `spawn_chests`, `to_net_state` |
| `src/render.rs` | Render `&` for P2; render `$` for chest; P2 HP bar in sidebar; `render_net_state`; disconnection overlay |
| `src/main.rs` | Add `mod` declarations; update `main()` to call menu; update single-player loop; add `run_loop_host`, `run_loop_client` |

---

## Task 1: Dependencies and Module Stubs

**Files:**
- Modify: `Cargo.toml`
- Create: `src/protocol.rs`
- Create: `src/net.rs`
- Create: `src/menu.rs`
- Modify: `src/main.rs`

- [ ] **Step 1: Add serde and bincode to `Cargo.toml`**

Replace the `[dependencies]` section:

```toml
[dependencies]
ratatui = "0.29"
crossterm = "0.28"
rand = "0.8"
serde = { version = "1", features = ["derive"] }
bincode = "1"
```

- [ ] **Step 2: Create `src/protocol.rs` stub**

```rust
// protocol module — populated in Task 2
```

- [ ] **Step 3: Create `src/net.rs` stub**

```rust
// net module — populated in Task 3
```

- [ ] **Step 4: Create `src/menu.rs` stub**

```rust
// menu module — populated in Task 9
```

- [ ] **Step 5: Add module declarations to `src/main.rs`**

Add these three lines directly after the existing `mod render;` line:

```rust
mod menu;
mod net;
mod protocol;
```

- [ ] **Step 6: Verify compilation**

```bash
cargo build
```

Expected: compiles cleanly (warnings about unused modules are fine).

---

## Task 2: Protocol Types

**Files:**
- Modify: `src/protocol.rs`

- [ ] **Step 1: Write the full contents of `src/protocol.rs`**

```rust
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub enum ClientMsg {
    Move(i32, i32),
    Quit,
}

#[derive(Serialize, Deserialize)]
pub enum ServerMsg {
    State(NetGameState),
    Disconnected,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct NetGameState {
    pub map_tiles: Vec<Vec<u8>>,
    pub player1: NetPlayer,
    pub player2: Option<NetPlayer>,
    pub monsters: Vec<NetMonster>,
    pub level: u8,
    pub status_msg: String,
    pub state: u8,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct NetPlayer {
    pub x: usize,
    pub y: usize,
    pub hp: i32,
    pub max_hp: i32,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct NetMonster {
    pub x: usize,
    pub y: usize,
    pub kind: u8,
    pub hp: i32,
    pub alive: bool,
}
```

- [ ] **Step 2: Verify compilation**

```bash
cargo build
```

Expected: compiles cleanly.

---

## Task 3: Net Framing

**Files:**
- Modify: `src/net.rs`

- [ ] **Step 1: Write the full contents of `src/net.rs`**

```rust
use std::io::{Read, Write};
use std::net::TcpStream;

pub fn send_msg<T: serde::Serialize>(stream: &mut TcpStream, msg: &T) -> std::io::Result<()> {
    let bytes = bincode::serialize(msg)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
    let len = bytes.len() as u32;
    stream.write_all(&len.to_le_bytes())?;
    stream.write_all(&bytes)?;
    Ok(())
}

pub fn recv_msg<T: serde::de::DeserializeOwned>(stream: &mut TcpStream) -> std::io::Result<T> {
    let mut len_buf = [0u8; 4];
    stream.read_exact(&mut len_buf)?;
    let len = u32::from_le_bytes(len_buf) as usize;
    let mut buf = vec![0u8; len];
    stream.read_exact(&mut buf)?;
    bincode::deserialize(&buf)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))
}
```

- [ ] **Step 2: Verify compilation**

```bash
cargo build
```

Expected: compiles cleanly.

---

## Task 4: Enum Repr Values and Tile::Chest

**Files:**
- Modify: `src/dungeon.rs`
- Modify: `src/entity.rs`

Adding `#[repr(u8)]` with explicit discriminants allows `tile as u8` casts in `to_net_state` and `From<u8>` impls for decoding on the client side.

- [ ] **Step 1: Update `Tile` in `src/dungeon.rs`**

Replace the existing `Tile` enum:

```rust
#[derive(Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum Tile {
    Wall = 0,
    Floor = 1,
    Corridor = 2,
    Stair = 3,
    Potion = 4,
    Chest = 5,
}

impl From<u8> for Tile {
    fn from(b: u8) -> Self {
        match b {
            1 => Tile::Floor,
            2 => Tile::Corridor,
            3 => Tile::Stair,
            4 => Tile::Potion,
            5 => Tile::Chest,
            _ => Tile::Wall,
        }
    }
}
```

- [ ] **Step 2: Update `Map::is_walkable` in `src/dungeon.rs` to include `Tile::Chest`**

```rust
pub fn is_walkable(&self, x: usize, y: usize) -> bool {
    if x >= MAP_WIDTH || y >= MAP_HEIGHT {
        return false;
    }
    matches!(
        self.tiles[y][x],
        Tile::Floor | Tile::Corridor | Tile::Stair | Tile::Potion | Tile::Chest
    )
}
```

- [ ] **Step 3: Update `MonsterKind` in `src/entity.rs`**

Replace the existing `MonsterKind` enum:

```rust
#[derive(Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum MonsterKind {
    Goblin = 0,
    Mage = 1,
    Rat = 2,
}

impl From<u8> for MonsterKind {
    fn from(b: u8) -> Self {
        match b {
            1 => MonsterKind::Mage,
            2 => MonsterKind::Rat,
            _ => MonsterKind::Goblin,
        }
    }
}
```

- [ ] **Step 4: Verify compilation**

```bash
cargo build
```

Expected: compiles cleanly.

---

## Task 5: Game Struct — Player 2, Multiplayer Flag, Spawn Updates

**Files:**
- Modify: `src/game.rs`

- [ ] **Step 1: Add `#[repr(u8)]` and `Disconnected` to `GameState` in `src/game.rs`**

Replace the existing `GameState` enum:

```rust
#[derive(PartialEq, Eq)]
#[repr(u8)]
pub enum GameState {
    Playing = 0,
    GameOver = 1,
    Victory = 2,
    ConfirmQuit = 3,
    Disconnected = 4,
}
```

- [ ] **Step 2: Add `player2` and `multiplayer` fields to `Game` in `src/game.rs`**

Replace the existing `Game` struct:

```rust
pub struct Game {
    pub state: GameState,
    pub map: Map,
    pub player: Player,
    pub player2: Option<Player>,
    pub monsters: Vec<Monster>,
    pub level: u8,
    pub status_msg: String,
    pub multiplayer: bool,
    pub rng: ThreadRng,
}
```

- [ ] **Step 3: Update `Game::new` to accept `multiplayer: bool` and spawn P2**

Replace the existing `Game::new` implementation:

```rust
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
    let monsters = spawn_monsters(&map, &mut rng, multiplayer);
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
        multiplayer,
        rng,
    }
}
```

- [ ] **Step 4: Update `next_level` to reposition both players**

Replace the existing `next_level` implementation:

```rust
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
    self.monsters = spawn_monsters(&self.map, &mut self.rng, self.multiplayer);
    spawn_potions(&mut self.map, &mut self.rng, &self.monsters);
    spawn_chests(&mut self.map, &mut self.rng, &self.monsters);
    self.status_msg = format!("Level {}. Deeper into Dagger Deep...", self.level);
}
```

- [ ] **Step 5: Update `spawn_monsters` to accept and apply `multiplayer: bool`**

Replace the existing `spawn_monsters` function:

```rust
fn spawn_monsters(map: &Map, rng: &mut impl Rng, multiplayer: bool) -> Vec<Monster> {
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
                if multiplayer {
                    // HP x1.5 rounded up
                    monster.hp = (monster.hp * 3 + 1) / 2;
                }
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
                        if multiplayer {
                            monster.hp = (monster.hp * 3 + 1) / 2;
                        }
                        monsters.push(monster);
                        break 'outer;
                    }
                }
            }
        }
    }
    monsters
}
```

- [ ] **Step 6: Add `spawn_chests` function to `src/game.rs`**

Add this function after `spawn_potions`:

```rust
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
        if matches!(map.get(x, y), Tile::Potion | Tile::Stair) {
            continue;
        }
        if monsters.iter().any(|m| m.x == x && m.y == y) {
            continue;
        }
        map.set(x, y, Tile::Chest);
        placed += 1;
    }
}
```

- [ ] **Step 7: Verify compilation**

```bash
cargo build
```

Expected: one error — `spawn_monsters` call in `try_spawn_stair` and other callers now need the `multiplayer` arg. Fix by updating the two calls in `Game::new` and `next_level` (already done above). The only remaining caller is the old `spawn_monsters(&map, &mut rng)` — both were updated in steps 3 and 4. Warnings about unused fields are fine.

---

## Task 6: Combat Difficulty Scaling

**Files:**
- Modify: `src/combat.rs`
- Modify: `src/game.rs` (update callers)

- [ ] **Step 1: Update `resolve_monster_attack` in `src/combat.rs`**

Replace the existing `resolve_monster_attack` function:

```rust
pub fn resolve_monster_attack(
    monster: &Monster,
    player: &mut Player,
    rng: &mut impl Rng,
    multiplayer: bool,
) -> (String, bool) {
    let hit_roll = roll_d6(rng);
    if hit_roll < 4 {
        return (format!("The {} missed you!", monster.kind.name()), false);
    }

    let damage = if multiplayer {
        roll_d6(rng).max(roll_d6(rng))
    } else {
        roll_d6(rng)
    };
    player.hp -= damage;

    let died = player.hp <= 0;
    let msg = format!(
        "The {} hits you for {} damage! ({} HP left)",
        monster.kind.name(),
        damage,
        player.hp.max(0)
    );
    (msg, died)
}
```

- [ ] **Step 2: Fix all callers of `resolve_monster_attack` in `src/game.rs`**

There are two call sites in `move_player`. Find each one and add `self.multiplayer` as the final argument:

```rust
// In move_player — monster counter-attack after player hits:
let (msg2, died) = crate::combat::resolve_monster_attack(
    &self.monsters[idx],
    &mut self.player,
    &mut self.rng,
    self.multiplayer,
);

// In move_monsters — monster steps onto player tile:
let (msg, died) = crate::combat::resolve_monster_attack(
    &self.monsters[i],
    &mut self.player,
    &mut self.rng,
    self.multiplayer,
);
```

- [ ] **Step 3: Verify compilation**

```bash
cargo build
```

Expected: compiles cleanly.

---

## Task 7: Game Logic — move_player Updates, move_player2, open_chest, move_monsters Update, to_net_state

**Files:**
- Modify: `src/game.rs`

- [ ] **Step 1: Update `move_player` to handle `Tile::Chest` and pass `multiplayer` to combat**

Replace the entire `move_player` implementation:

```rust
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

    if let Some(idx) = self.monsters.iter().position(|m| m.is_alive() && m.x == nx && m.y == ny) {
        let msg = crate::combat::resolve_player_attack(
            &mut self.player,
            &mut self.monsters[idx],
            &mut self.rng,
        );
        self.status_msg = msg;

        if self.monsters[idx].is_alive() {
            let (msg2, died) = crate::combat::resolve_monster_attack(
                &self.monsters[idx],
                &mut self.player,
                &mut self.rng,
                self.multiplayer,
            );
            self.status_msg = msg2;
            if died {
                self.state = GameState::GameOver;
                return;
            }
        } else {
            self.try_spawn_stair();
        }
        return;
    }

    self.player.x = nx;
    self.player.y = ny;

    if self.map.get(nx, ny) == Tile::Potion {
        self.player.hp = self.player.max_hp;
        self.map.set(nx, ny, Tile::Floor);
        self.status_msg = format!("You drank a potion and feel restored! ({} HP)", self.player.hp);
        return;
    }

    if self.map.get(nx, ny) == Tile::Chest {
        self.open_chest(false);
        return;
    }

    if self.map.get(nx, ny) == Tile::Stair {
        if self.level == 10 {
            self.state = GameState::Victory;
        } else {
            self.status_msg = String::from("The stairs lead deeper into the dungeon...");
            self.next_level();
        }
        return;
    }
}
```

- [ ] **Step 2: Add `move_player2` to `src/game.rs`**

Add this method after `move_player`:

```rust
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
        let msg = crate::combat::resolve_player_attack(p2, &mut self.monsters[idx], &mut self.rng);
        self.status_msg = msg;

        if self.monsters[idx].is_alive() {
            let (msg2, died) = crate::combat::resolve_monster_attack(
                &self.monsters[idx],
                self.player2.as_mut().unwrap(),
                &mut self.rng,
                self.multiplayer,
            );
            self.status_msg = msg2;
            if died {
                self.respawn_player2();
            }
        } else {
            self.try_spawn_stair();
        }
        return;
    }

    let p2 = self.player2.as_mut().unwrap();
    p2.x = nx;
    p2.y = ny;

    if self.map.get(nx, ny) == Tile::Potion {
        let max = self.player2.as_ref().unwrap().max_hp;
        self.player2.as_mut().unwrap().hp = max;
        self.map.set(nx, ny, Tile::Floor);
        self.status_msg = format!("P2 drank a potion and feels restored! ({} HP)", max);
        return;
    }

    if self.map.get(nx, ny) == Tile::Chest {
        self.open_chest(true);
        return;
    }

    if self.map.get(nx, ny) == Tile::Stair {
        if self.level == 10 {
            self.state = GameState::Victory;
        } else {
            self.status_msg = String::from("The stairs lead deeper into the dungeon...");
            self.next_level();
        }
        return;
    }
}
```

- [ ] **Step 3: Add `respawn_player2` helper to `src/game.rs`**

Add this private method:

```rust
fn respawn_player2(&mut self) {
    let (rx, ry) = self.map.rooms.first().map(|r| r.center()).unwrap_or((1, 1));
    if let Some(p2) = self.player2.as_mut() {
        p2.x = rx + 1;
        p2.y = ry;
        p2.hp = p2.max_hp / 2;
    }
    self.status_msg = String::from("P2 was defeated and respawned!");
}
```

- [ ] **Step 4: Add `open_chest` to `src/game.rs`**

Add this public method:

```rust
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
        self.status_msg = String::from("The chest contains food! Both players are fully healed!");
    } else if roll < 0.65 {
        let damage = crate::combat::roll_d6(&mut self.rng);
        if is_p2 {
            let p2 = self.player2.as_mut().unwrap();
            p2.hp -= damage;
            let hp_left = p2.hp.max(0);
            if p2.hp <= 0 {
                drop(p2);
                self.respawn_player2();
                self.status_msg = String::from("The chest was a trap! P2 was defeated and respawned!");
            } else {
                self.status_msg = format!("The chest was a trap! P2 takes {} damage! ({} HP left)", damage, hp_left);
            }
        } else {
            self.player.hp -= damage;
            if self.player.hp <= 0 {
                self.state = GameState::GameOver;
                self.status_msg = String::from("The chest was a trap! You have perished in Dagger Deep.");
            } else {
                self.status_msg = format!(
                    "The chest was a trap! You take {} damage! ({} HP left)",
                    damage, self.player.hp.max(0)
                );
            }
        }
    } else {
        self.status_msg = String::from("The chest was empty.");
    }
    self.map.set(nx, ny, Tile::Floor);
}
```

- [ ] **Step 5: Update `move_monsters` to also attack P2 if a monster steps onto P2's tile**

Replace the entire `move_monsters` implementation:

```rust
pub fn move_monsters(&mut self) {
    let px = self.player.x;
    let py = self.player.y;

    for i in 0..self.monsters.len() {
        if !self.monsters[i].is_alive() {
            continue;
        }

        let mx = self.monsters[i].x as i32;
        let my = self.monsters[i].y as i32;
        let dx = px as i32 - mx;
        let dy = py as i32 - my;

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
                self.state = GameState::GameOver;
                return;
            }
        } else if self.player2.as_ref().map_or(false, |p| p.x == nx && p.y == ny) {
            let (msg, died) = crate::combat::resolve_monster_attack(
                &self.monsters[i],
                self.player2.as_mut().unwrap(),
                &mut self.rng,
                self.multiplayer,
            );
            self.status_msg = msg;
            if died {
                self.respawn_player2();
            }
        } else {
            self.monsters[i].x = nx;
            self.monsters[i].y = ny;
        }
    }
}
```

- [ ] **Step 6: Add `to_net_state` to `src/game.rs`**

Add this public method. It must be called after the `use` imports that already reference `crate::protocol`:

```rust
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
            alive: m.alive,
        }).collect(),
        level: self.level,
        status_msg: self.status_msg.clone(),
        state: self.state as u8,
    }
}
```

- [ ] **Step 7: Verify compilation**

```bash
cargo build
```

Expected: compiles cleanly. If there are borrow errors in `open_chest`, ensure the `drop(p2)` line before `respawn_player2()` is present in the P2-dies-to-trap branch.

---

## Task 8: Render Updates

**Files:**
- Modify: `src/render.rs`

- [ ] **Step 1: Add `Tile::Chest` rendering and P2 rendering to `render_dungeon`**

Inside the `render_dungeon` function, add P2 rendering after the P1 check and before the monster check:

```rust
// After the player1 block and before the monster block:
if let Some(ref p2) = game.player2 {
    if p2.x == x && p2.y == y {
        spans.push(Span::styled(
            "&",
            Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD),
        ));
        continue;
    }
}
```

In the tile match arm, add the `Chest` variant:

```rust
let (ch, style) = match game.map.get(x, y) {
    Tile::Wall => ('#', Style::default().fg(Color::DarkGray).bg(Color::Black)),
    Tile::Floor | Tile::Corridor => {
        ('.', Style::default().fg(Color::DarkGray).bg(Color::Black))
    }
    Tile::Stair => ('>', Style::default().fg(Color::Yellow).bg(Color::Black)),
    Tile::Potion => ('P', Style::default().fg(Color::Green).bg(Color::Black)),
    Tile::Chest => ('$', Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD).bg(Color::Black)),
};
```

- [ ] **Step 2: Update `render_sidebar` to show P2 HP bar when present**

Replace the entire `render_sidebar` function:

```rust
fn render_sidebar(frame: &mut Frame, game: &Game, area: Rect) {
    let hp_color = |hp: i32, max_hp: i32| {
        if hp <= max_hp / 2 { Color::Red } else { Color::Green }
    };

    let mut lines = vec![
        Line::from(Span::styled(
            "PLAYER 1",
            Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD),
        )),
        Line::from(vec![
            Span::raw("HP: "),
            Span::styled(
                format!("{}/{}", game.player.hp.max(0), game.player.max_hp),
                Style::default().fg(hp_color(game.player.hp, game.player.max_hp)),
            ),
        ]),
    ];

    if let Some(ref p2) = game.player2 {
        lines.push(Line::raw(""));
        lines.push(Line::from(Span::styled(
            "PLAYER 2",
            Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD),
        )));
        lines.push(Line::from(vec![
            Span::raw("HP: "),
            Span::styled(
                format!("{}/{}", p2.hp.max(0), p2.max_hp),
                Style::default().fg(hp_color(p2.hp, p2.max_hp)),
            ),
        ]));
    }

    lines.push(Line::raw(""));
    lines.push(Line::from(Span::styled(
        "LEGEND",
        Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD),
    )));
    lines.extend([
        Line::from(vec![Span::styled("@ ", Style::default().fg(Color::White)), Span::raw("P1")]),
        Line::from(vec![Span::styled("& ", Style::default().fg(Color::Cyan)), Span::raw("P2")]),
        Line::from(vec![Span::styled("G ", Style::default().fg(Color::Blue)), Span::raw("Goblin")]),
        Line::from(vec![Span::styled("M ", Style::default().fg(Color::Red)), Span::raw("Mage")]),
        Line::from(vec![Span::styled("R ", Style::default().fg(Color::Rgb(139, 90, 43))), Span::raw("Rat")]),
        Line::from(vec![Span::styled("P ", Style::default().fg(Color::Green)), Span::raw("Potion")]),
        Line::from(vec![Span::styled("$ ", Style::default().fg(Color::Yellow)), Span::raw("Chest")]),
        Line::from(vec![Span::styled("> ", Style::default().fg(Color::Yellow)), Span::raw("Stairs")]),
    ]);

    let para = Paragraph::new(lines)
        .block(
            Block::default()
                .borders(Borders::LEFT)
                .border_style(Style::default().fg(Color::DarkGray)),
        )
        .style(Style::default().bg(Color::Black));
    frame.render_widget(para, area);
}
```

- [ ] **Step 3: Add `GameState::Disconnected` overlay to `render_game`**

In `render_game`, add a third overlay branch after the Victory check:

```rust
} else if game.state == GameState::Disconnected {
    render_overlay(
        frame,
        area,
        "DISCONNECTED",
        "Connection lost.",
        Color::Gray,
    );
}
```

- [ ] **Step 4: Add `render_net_state` function to `src/render.rs`**

Add the following imports at the top of render.rs (merge with the existing `use crate::dungeon` line):

```rust
use crate::dungeon::{MAP_HEIGHT, MAP_WIDTH, Tile};
use crate::entity::MonsterKind;
use crate::game::{Game, GameState};
use crate::protocol::NetGameState;
```

Add this function before `render_top_bar`:

```rust
pub fn render_net_state(frame: &mut Frame, state: &NetGameState) {
    let game_width = (MAP_WIDTH + 19) as u16;
    let game_height = (MAP_HEIGHT + 2) as u16;
    let area = Rect { x: 0, y: 0, width: game_width, height: game_height };

    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1),
            Constraint::Length(MAP_HEIGHT as u16),
            Constraint::Length(1),
        ])
        .split(area);

    // Top bar
    let title = format!("⚔  Return to Dagger Deep  ⚔        Level: {}/10", state.level);
    let para = Paragraph::new(title)
        .alignment(Alignment::Center)
        .style(Style::default().fg(Color::White).bg(Color::DarkGray).add_modifier(Modifier::BOLD));
    frame.render_widget(para, rows[0]);

    let cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Length(MAP_WIDTH as u16), Constraint::Length(19)])
        .split(rows[1]);

    // Dungeon
    let mut lines: Vec<Line> = Vec::new();
    for y in 0..MAP_HEIGHT {
        let mut spans: Vec<Span> = Vec::new();
        for x in 0..MAP_WIDTH {
            if state.player1.x == x && state.player1.y == y {
                spans.push(Span::styled("@", Style::default().fg(Color::White).add_modifier(Modifier::BOLD)));
                continue;
            }
            if let Some(ref p2) = state.player2 {
                if p2.x == x && p2.y == y {
                    spans.push(Span::styled("&", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)));
                    continue;
                }
            }
            if let Some(m) = state.monsters.iter().find(|m| m.alive && m.x == x && m.y == y) {
                let kind = MonsterKind::from(m.kind);
                let color = match kind {
                    MonsterKind::Goblin => Color::Blue,
                    MonsterKind::Mage => Color::Red,
                    MonsterKind::Rat => Color::LightCyan,
                };
                spans.push(Span::styled(
                    kind.char().to_string(),
                    Style::default().fg(color).add_modifier(Modifier::BOLD),
                ));
                continue;
            }
            let tile = Tile::from(state.map_tiles[y][x]);
            let (ch, style) = match tile {
                Tile::Wall => ('#', Style::default().fg(Color::DarkGray).bg(Color::Black)),
                Tile::Floor | Tile::Corridor => ('.', Style::default().fg(Color::DarkGray).bg(Color::Black)),
                Tile::Stair => ('>', Style::default().fg(Color::Yellow).bg(Color::Black)),
                Tile::Potion => ('P', Style::default().fg(Color::Green).bg(Color::Black)),
                Tile::Chest => ('$', Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD).bg(Color::Black)),
            };
            spans.push(Span::styled(ch.to_string(), style));
        }
        lines.push(Line::from(spans));
    }
    let para = Paragraph::new(lines)
        .block(Block::default().borders(Borders::NONE))
        .style(Style::default().bg(Color::Black));
    frame.render_widget(para, cols[0]);

    // Sidebar
    let hp_color = |hp: i32, max_hp: i32| {
        if hp <= max_hp / 2 { Color::Red } else { Color::Green }
    };
    let mut sidebar_lines = vec![
        Line::from(Span::styled("PLAYER 1", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD))),
        Line::from(vec![
            Span::raw("HP: "),
            Span::styled(
                format!("{}/{}", state.player1.hp.max(0), state.player1.max_hp),
                Style::default().fg(hp_color(state.player1.hp, state.player1.max_hp)),
            ),
        ]),
    ];
    if let Some(ref p2) = state.player2 {
        sidebar_lines.push(Line::raw(""));
        sidebar_lines.push(Line::from(Span::styled("PLAYER 2", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD))));
        sidebar_lines.push(Line::from(vec![
            Span::raw("HP: "),
            Span::styled(
                format!("{}/{}", p2.hp.max(0), p2.max_hp),
                Style::default().fg(hp_color(p2.hp, p2.max_hp)),
            ),
        ]));
    }
    sidebar_lines.extend([
        Line::raw(""),
        Line::from(Span::styled("LEGEND", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD))),
        Line::from(vec![Span::styled("@ ", Style::default().fg(Color::White)), Span::raw("P1")]),
        Line::from(vec![Span::styled("& ", Style::default().fg(Color::Cyan)), Span::raw("P2")]),
        Line::from(vec![Span::styled("G ", Style::default().fg(Color::Blue)), Span::raw("Goblin")]),
        Line::from(vec![Span::styled("M ", Style::default().fg(Color::Red)), Span::raw("Mage")]),
        Line::from(vec![Span::styled("$ ", Style::default().fg(Color::Yellow)), Span::raw("Chest")]),
        Line::from(vec![Span::styled("> ", Style::default().fg(Color::Yellow)), Span::raw("Stairs")]),
    ]);
    let para = Paragraph::new(sidebar_lines)
        .block(Block::default().borders(Borders::LEFT).border_style(Style::default().fg(Color::DarkGray)))
        .style(Style::default().bg(Color::Black));
    frame.render_widget(para, cols[1]);

    // Status bar
    let para = Paragraph::new(state.status_msg.clone())
        .style(Style::default().fg(Color::White).add_modifier(Modifier::BOLD).bg(Color::Black));
    frame.render_widget(para, rows[2]);

    // Overlays
    match state.state {
        1 => render_overlay(frame, area, "GAME OVER", "You have perished in Dagger Deep.", Color::Red),
        2 => render_overlay(frame, area, "VICTORY!", "You have conquered Dagger Deep!", Color::Yellow),
        4 => render_overlay(frame, area, "DISCONNECTED", "Connection lost.", Color::Gray),
        _ => {}
    }
}
```

- [ ] **Step 5: Verify compilation**

```bash
cargo build
```

Expected: compiles cleanly. If `NetGameState` import causes a conflict, ensure `use crate::protocol::NetGameState;` is present at the top of `render.rs`.

---

## Task 9: TUI Menu

**Files:**
- Modify: `src/menu.rs`

- [ ] **Step 1: Write the full contents of `src/menu.rs`**

```rust
use std::net::{TcpListener, TcpStream};
use std::sync::mpsc;
use std::thread;
use std::time::Duration;

use crossterm::event::{self, Event, KeyCode};
use ratatui::{
    Frame,
    backend::CrosstermBackend,
    layout::{Alignment, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Terminal,
};
use std::io;

pub enum MenuResult {
    SinglePlayer,
    Host(TcpStream),
    Join(TcpStream),
    Quit,
}

pub fn run_menu(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
) -> Result<MenuResult, io::Error> {
    loop {
        terminal.draw(render_main_menu)?;

        if let Event::Key(key) = event::read()? {
            match key.code {
                KeyCode::Char('s') | KeyCode::Char('S') => return Ok(MenuResult::SinglePlayer),
                KeyCode::Char('h') | KeyCode::Char('H') => {
                    if let Some(stream) = run_host_screen(terminal)? {
                        return Ok(MenuResult::Host(stream));
                    }
                }
                KeyCode::Char('j') | KeyCode::Char('J') => {
                    if let Some(stream) = run_join_screen(terminal)? {
                        return Ok(MenuResult::Join(stream));
                    }
                }
                KeyCode::Char('q') | KeyCode::Char('Q') | KeyCode::Esc => {
                    return Ok(MenuResult::Quit);
                }
                _ => {}
            }
        }
    }
}

fn render_main_menu(frame: &mut Frame) {
    let area = frame.area();
    let lines = vec![
        Line::raw(""),
        Line::raw(""),
        Line::from(Span::styled(
            "⚔  Return to Dagger Deep  ⚔",
            Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD),
        )),
        Line::raw(""),
        Line::from(Span::styled("[S]  Single Player", Style::default().fg(Color::White))),
        Line::from(Span::styled("[H]  Host Game", Style::default().fg(Color::White))),
        Line::from(Span::styled("[J]  Join Game", Style::default().fg(Color::White))),
        Line::raw(""),
        Line::from(Span::styled("[Q]  Quit", Style::default().fg(Color::DarkGray))),
    ];
    let para = Paragraph::new(lines)
        .alignment(Alignment::Center)
        .block(Block::default().borders(Borders::ALL).border_style(Style::default().fg(Color::DarkGray)))
        .style(Style::default().bg(Color::Black));
    frame.render_widget(para, area);
}

fn run_host_screen(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
) -> Result<Option<TcpStream>, io::Error> {
    let listener = TcpListener::bind("0.0.0.0:4444")
        .map_err(|e| io::Error::new(e.kind(), format!("Could not bind port 4444: {}", e)))?;

    let (tx, rx) = mpsc::channel::<TcpStream>();
    thread::spawn(move || {
        if let Ok((stream, _)) = listener.accept() {
            let _ = tx.send(stream);
        }
    });

    let spinner = ['|', '/', '-', '\\'];
    let mut spin_idx: usize = 0;

    loop {
        let ch = spinner[spin_idx % 4];
        terminal.draw(|frame| render_waiting(frame, ch))?;
        spin_idx += 1;

        if event::poll(Duration::from_millis(120))? {
            if let Event::Key(key) = event::read()? {
                if key.code == KeyCode::Esc {
                    return Ok(None);
                }
            }
        }

        if let Ok(stream) = rx.try_recv() {
            return Ok(Some(stream));
        }
    }
}

fn render_waiting(frame: &mut Frame, spinner: char) {
    let area = frame.area();
    let lines = vec![
        Line::raw(""),
        Line::raw(""),
        Line::from(Span::styled(
            "Waiting for player 2 on port 4444...",
            Style::default().fg(Color::Cyan),
        )),
        Line::from(Span::styled(
            spinner.to_string(),
            Style::default().fg(Color::White).add_modifier(Modifier::BOLD),
        )),
        Line::raw(""),
        Line::from(Span::styled("[Esc] Cancel", Style::default().fg(Color::DarkGray))),
    ];
    let para = Paragraph::new(lines)
        .alignment(Alignment::Center)
        .block(Block::default().borders(Borders::ALL).border_style(Style::default().fg(Color::DarkGray)))
        .style(Style::default().bg(Color::Black));
    frame.render_widget(para, area);
}

fn run_join_screen(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
) -> Result<Option<TcpStream>, io::Error> {
    let mut input = String::new();
    let mut error_msg = String::new();

    loop {
        let input_clone = input.clone();
        let err_clone = error_msg.clone();
        terminal.draw(move |frame| render_join(frame, &input_clone, &err_clone))?;

        if let Event::Key(key) = event::read()? {
            match key.code {
                KeyCode::Esc => return Ok(None),
                KeyCode::Enter => {
                    if input.is_empty() {
                        error_msg = String::from("Enter an IP address.");
                        continue;
                    }
                    let addr = format!("{}:4444", input.trim());
                    match TcpStream::connect(&addr) {
                        Ok(stream) => return Ok(Some(stream)),
                        Err(e) => {
                            error_msg = format!("Connection failed: {}", e);
                        }
                    }
                }
                KeyCode::Backspace => {
                    input.pop();
                    error_msg.clear();
                }
                KeyCode::Char(c) => {
                    input.push(c);
                    error_msg.clear();
                }
                _ => {}
            }
        }
    }
}

fn render_join(frame: &mut Frame, input: &str, error: &str) {
    let area = frame.area();
    let mut lines = vec![
        Line::raw(""),
        Line::raw(""),
        Line::from(Span::styled(
            "Enter host IP address:",
            Style::default().fg(Color::Cyan),
        )),
        Line::from(Span::styled(
            format!("> {}_", input),
            Style::default().fg(Color::White).add_modifier(Modifier::BOLD),
        )),
        Line::raw(""),
        Line::from(Span::styled("[Enter] Connect   [Esc] Back", Style::default().fg(Color::DarkGray))),
    ];
    if !error.is_empty() {
        lines.push(Line::raw(""));
        lines.push(Line::from(Span::styled(error.to_string(), Style::default().fg(Color::Red))));
    }
    let para = Paragraph::new(lines)
        .alignment(Alignment::Center)
        .block(Block::default().borders(Borders::ALL).border_style(Style::default().fg(Color::DarkGray)))
        .style(Style::default().bg(Color::Black));
    frame.render_widget(para, area);
}
```

- [ ] **Step 2: Verify compilation**

```bash
cargo build
```

Expected: compiles cleanly.

---

## Task 10: Main — Host Loop, Client Loop, and Wiring

**Files:**
- Modify: `src/main.rs`

- [ ] **Step 1: Replace the full contents of `src/main.rs`**

```rust
mod combat;
mod dungeon;
mod entity;
mod game;
mod menu;
mod net;
mod protocol;
mod render;

use std::net::TcpStream;
use std::sync::mpsc;
use std::thread;
use std::time::Duration;
use std::io::ErrorKind;

use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyModifiers},
    execute,
    terminal::{
        disable_raw_mode, enable_raw_mode, size, EnterAlternateScreen, LeaveAlternateScreen,
    },
};
use ratatui::{backend::CrosstermBackend, Terminal};
use std::io;

use game::{Game, GameState};
use menu::MenuResult;
use protocol::{ClientMsg, ServerMsg};

fn check_terminal_size() -> Result<(), String> {
    let (cols, rows) = size().map_err(|e| e.to_string())?;
    if cols < 100 || rows < 30 {
        Err(format!(
            "Terminal too small: {}×{}. Minimum required: 100×30.",
            cols, rows
        ))
    } else {
        Ok(())
    }
}

fn main() -> Result<(), io::Error> {
    if let Err(msg) = check_terminal_size() {
        eprintln!("Error: {}", msg);
        std::process::exit(1);
    }

    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let result = run(&mut terminal);

    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    result
}

fn run(terminal: &mut Terminal<CrosstermBackend<io::Stdout>>) -> Result<(), io::Error> {
    match menu::run_menu(terminal)? {
        MenuResult::Quit => {}
        MenuResult::SinglePlayer => {
            let mut game = Game::new(false);
            run_loop(terminal, &mut game)?;
        }
        MenuResult::Host(stream) => {
            let mut game = Game::new(true);
            run_loop_host(terminal, &mut game, stream)?;
        }
        MenuResult::Join(stream) => {
            run_loop_client(terminal, stream)?;
        }
    }
    Ok(())
}

fn run_loop(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    game: &mut Game,
) -> Result<(), io::Error> {
    loop {
        terminal.draw(|frame| render::render_game(frame, game))?;

        if let Event::Key(key) = event::read()? {
            match game.state {
                GameState::Playing => match key.code {
                    KeyCode::Esc => game.state = GameState::ConfirmQuit,
                    KeyCode::Up => {
                        game.move_player(0, -1);
                        if game.state == GameState::Playing {
                            game.move_monsters();
                        }
                    }
                    KeyCode::Down => {
                        game.move_player(0, 1);
                        if game.state == GameState::Playing {
                            game.move_monsters();
                        }
                    }
                    KeyCode::Left => {
                        game.move_player(-1, 0);
                        if game.state == GameState::Playing {
                            game.move_monsters();
                        }
                    }
                    KeyCode::Right => {
                        game.move_player(1, 0);
                        if game.state == GameState::Playing {
                            game.move_monsters();
                        }
                    }
                    _ => {}
                },
                GameState::ConfirmQuit => match key.code {
                    KeyCode::Char('y') | KeyCode::Char('Y') | KeyCode::Enter => break,
                    KeyCode::Char('n') | KeyCode::Char('N') | KeyCode::Esc => {
                        game.state = GameState::Playing;
                    }
                    KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => break,
                    _ => {}
                },
                GameState::GameOver | GameState::Victory | GameState::Disconnected => break,
            }
        }
    }
    Ok(())
}

fn run_loop_host(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    game: &mut Game,
    stream: TcpStream,
) -> Result<(), io::Error> {
    // Clone for the recv thread; main thread keeps the original for writing.
    let read_stream = stream.try_clone()?;
    let mut write_stream = stream;

    let (tx, rx) = mpsc::channel::<ClientMsg>();
    thread::spawn(move || {
        let mut s = read_stream;
        loop {
            match net::recv_msg::<ClientMsg>(&mut s) {
                Ok(msg) => {
                    if tx.send(msg).is_err() {
                        break;
                    }
                }
                Err(_) => {
                    let _ = tx.send(ClientMsg::Quit);
                    break;
                }
            }
        }
    });

    loop {
        terminal.draw(|frame| render::render_game(frame, game))?;

        // Poll for P1 input (50ms timeout so we still drain the P2 channel promptly)
        if event::poll(Duration::from_millis(50))? {
            if let Event::Key(key) = event::read()? {
                match game.state {
                    GameState::Playing => match key.code {
                        KeyCode::Esc => game.state = GameState::ConfirmQuit,
                        KeyCode::Up => game.move_player(0, -1),
                        KeyCode::Down => game.move_player(0, 1),
                        KeyCode::Left => game.move_player(-1, 0),
                        KeyCode::Right => game.move_player(1, 0),
                        _ => {}
                    },
                    GameState::ConfirmQuit => match key.code {
                        KeyCode::Char('y') | KeyCode::Char('Y') | KeyCode::Enter => break,
                        KeyCode::Char('n') | KeyCode::Char('N') | KeyCode::Esc => {
                            game.state = GameState::Playing;
                        }
                        KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => break,
                        _ => {}
                    },
                    GameState::GameOver | GameState::Victory | GameState::Disconnected => break,
                }
            }
        }

        // Drain P2 input channel
        while let Ok(msg) = rx.try_recv() {
            match msg {
                ClientMsg::Move(dx, dy) => {
                    if game.state == GameState::Playing {
                        game.move_player2(dx, dy);
                    }
                }
                ClientMsg::Quit => {
                    game.player2 = None;
                    game.status_msg = String::from("P2 quit. Continuing solo.");
                }
            }
        }

        // Monsters move after both players have had a chance to act
        if game.state == GameState::Playing {
            game.move_monsters();
        }

        // Send snapshot to client
        let net_state = game.to_net_state();
        if net::send_msg(&mut write_stream, &ServerMsg::State(net_state)).is_err() {
            game.player2 = None;
            game.status_msg = String::from("P2 disconnected. Continuing solo.");
        }

        // The regular send above already delivered the final state; just exit.
        if matches!(game.state, GameState::GameOver | GameState::Victory) {
            break;
        }
    }
    Ok(())
}

fn run_loop_client(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    stream: TcpStream,
) -> Result<(), io::Error> {
    let read_stream = stream.try_clone()?;
    let mut write_stream = stream;

    let (tx, rx) = mpsc::channel::<ServerMsg>();
    thread::spawn(move || {
        let mut s = read_stream;
        loop {
            match net::recv_msg::<ServerMsg>(&mut s) {
                Ok(msg) => {
                    if tx.send(msg).is_err() {
                        break;
                    }
                }
                Err(e) if e.kind() == ErrorKind::WouldBlock || e.kind() == ErrorKind::TimedOut => {
                    continue;
                }
                Err(_) => {
                    let _ = tx.send(ServerMsg::Disconnected);
                    break;
                }
            }
        }
    });

    let mut net_state: Option<protocol::NetGameState> = None;
    let mut disconnected = false;

    loop {
        terminal.draw(|frame| {
            if disconnected {
                render::render_disconnected(frame);
            } else if let Some(ref s) = net_state {
                render::render_net_state(frame, s);
            } else {
                render::render_connecting(frame);
            }
        })?;

        // Drain server channel first (non-blocking)
        while let Ok(msg) = rx.try_recv() {
            match msg {
                ServerMsg::State(s) => {
                    net_state = Some(s);
                }
                ServerMsg::Disconnected => {
                    disconnected = true;
                }
            }
        }

        // If game ended on host, show final state until keypress
        if let Some(ref s) = net_state {
            if s.state == 1 || s.state == 2 {
                // GameOver or Victory — wait for any key
                if event::poll(Duration::from_millis(50))? {
                    event::read()?;
                    break;
                }
                continue;
            }
        }

        if disconnected {
            event::poll(Duration::from_millis(100))?;
            event::read()?;
            break;
        }

        // Poll for client input
        if event::poll(Duration::from_millis(50))? {
            if let Event::Key(key) = event::read()? {
                match key.code {
                    KeyCode::Esc => {
                        let _ = net::send_msg(&mut write_stream, &ClientMsg::Quit);
                        break;
                    }
                    KeyCode::Up => {
                        let _ = net::send_msg(&mut write_stream, &ClientMsg::Move(0, -1));
                    }
                    KeyCode::Down => {
                        let _ = net::send_msg(&mut write_stream, &ClientMsg::Move(0, 1));
                    }
                    KeyCode::Left => {
                        let _ = net::send_msg(&mut write_stream, &ClientMsg::Move(-1, 0));
                    }
                    KeyCode::Right => {
                        let _ = net::send_msg(&mut write_stream, &ClientMsg::Move(1, 0));
                    }
                    _ => {}
                }
            }
        }
    }
    Ok(())
}
```

- [ ] **Step 2: Add `render_connecting` and `render_disconnected` to `src/render.rs`**

Add these two functions to `render.rs`:

```rust
pub fn render_connecting(frame: &mut Frame) {
    let area = frame.area();
    let lines = vec![
        Line::raw(""),
        Line::raw(""),
        Line::from(Span::styled(
            "Connecting to host...",
            Style::default().fg(Color::Cyan),
        )),
    ];
    let para = Paragraph::new(lines)
        .alignment(Alignment::Center)
        .style(Style::default().bg(Color::Black));
    frame.render_widget(para, area);
}

pub fn render_disconnected(frame: &mut Frame) {
    let area = frame.area();
    let lines = vec![
        Line::raw(""),
        Line::raw(""),
        Line::from(Span::styled(
            "Connection lost.",
            Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
        )),
        Line::from(Span::styled(
            "Press any key to exit.",
            Style::default().fg(Color::Gray),
        )),
    ];
    let para = Paragraph::new(lines)
        .alignment(Alignment::Center)
        .style(Style::default().bg(Color::Black));
    frame.render_widget(para, area);
}
```

- [ ] **Step 3: Verify full compilation**

```bash
cargo build
```

Expected: zero errors. Warnings about unused variables or imports are acceptable.

---

## Task 11: Integration Verification

**Files:** None (verification only)

- [ ] **Step 1: Verify single-player still works**

```bash
cargo run
```

Press `S` at the menu. Verify the game launches, `@` appears, arrow keys move, monsters chase, potions heal, chests appear as `$`, ESC shows quit modal. Verify 10 levels and victory still work.

- [ ] **Step 2: Verify host mode starts**

```bash
cargo run
```

Press `H`. Verify the waiting screen appears with port `4444` and a spinner. Press `Esc` to return to menu.

- [ ] **Step 3: Verify join screen**

Press `J`. Verify IP input field appears. Type some characters, verify they display. Press `Backspace`, verify they delete. Enter an invalid address, press Enter, verify an error message appears. Press `Esc` to return.

- [ ] **Step 4: Two-terminal multiplayer test**

Open two terminal windows. In the first:

```bash
cargo run
# Press H — wait for P2
```

In the second:

```bash
cargo run
# Press J — type 127.0.0.1 — press Enter
```

Verify: both terminals show the same dungeon. P1 (`@`) and P2 (`&`) are visible. Arrow keys on each terminal move the respective player. Monsters chase P1. Status bar updates on both terminals after each move.

- [ ] **Step 5: Verify chest behaviour**

Walk either player onto a `$` tile. Verify one of three status messages appears:
- "The chest contains food! Both players are fully healed!"
- "The chest was a trap! ..."
- "The chest was empty."

Verify the `$` tile disappears after opening.

- [ ] **Step 6: Verify P2 respawn**

Let P2 take damage until HP reaches 0 (either from monster or trap). Verify P2 respawns at starting room with `max_hp / 2` HP. Verify P1 is unaffected.

- [ ] **Step 7: Verify stair sync**

Kill all monsters on level 1. Verify `>` appears. Step either player onto `>`. Verify both players advance to level 2 together.

- [ ] **Step 8: Verify disconnect handling**

Start a host+client session. Close the client terminal with Ctrl+C. Verify the host receives "P2 disconnected. Continuing solo." and continues normally as single-player.
