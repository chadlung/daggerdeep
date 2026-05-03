# Return to Dagger Deep — Design Spec
**Date:** 2026-03-21
**Project:** tui-rogue
**Stack:** Rust, Ratatui, Crossterm, Rand

---

## Overview

A classic terminal roguelike with 10 dungeon levels. The player moves through procedurally generated dungeons, fights monsters, collects potions, and descends via staircases. Clearing all monsters on level 10 wins the game.

---

## Visual Design Decisions

- **Layout:** Sidebar panel (B) — title bar on top, dungeon + sidebar side-by-side, status bar on bottom
- **Dungeon style:** Classic Rogue (A) — `#` walls, `.` floors, black background, grey walls

---

## System Requirements

**Minimum terminal size:** 110 columns × 44 rows. If the terminal is smaller on startup, the game exits with a helpful error message rather than rendering incorrectly.

---

## Module Structure

```
src/
├── main.rs          # Entry point, terminal setup, event loop
├── game.rs          # GameState enum, top-level game loop logic
├── dungeon.rs       # Map generation (rooms + corridors), tile types
├── entity.rs        # Player and Monster structs, movement logic
├── combat.rs        # Attack resolution, dice rolls
└── render.rs        # All Ratatui rendering
```

### Dependencies (`Cargo.toml`)

```toml
[dependencies]
ratatui = "0.29"
crossterm = "0.28"
rand = "0.8"
```

---

## Game State Machine

```rust
enum GameState {
    Playing,
    GameOver,
    Victory,
}
```

The main loop runs each frame until `GameState` is `GameOver` or `Victory`, or the player presses Escape.

- **Playing:** normal gameplay turn sequence (see Turn Order below)
- **GameOver:** player HP ≤ 0; renders "Game Over" screen, any key exits the process
- **Victory:** all monsters cleared on level 10 and player steps on `>`; renders win screen, any key exits the process
- **Escape:** exits the game immediately from `Playing` state only; on GameOver/Victory screens any key exits

### Turn Order

Each game turn executes in this exact order:

1. Process player input (movement or wait)
2. If player moved into a monster tile: resolve player-attacks-monster combat
3. Move all living monsters one step toward the player
4. For any monster that moved onto the player's tile: resolve monster-attacks-player combat
5. Check win/lose conditions
6. Render

If player HP drops to ≤ 0 at any point during step 4, immediately set `GameState::GameOver`, stop processing remaining monster attacks, and render the Game Over screen on the next frame.

### Level Progression

The `Game` struct holds `current_level: u8` (1–10).

- When all 3 monsters are dead, place staircase `>` at a random walkable floor tile that is not the player's current tile and not occupied by a potion. If no such tile exists (extremely unlikely given 90×40 map), place it on the player's tile as an emergency fallback. Show status message: `"The way down is revealed..."`.
- Stepping on `>` on levels 1–9: increment level, regenerate dungeon, place 3 fresh monsters, reset potion spawns.
- Stepping on `>` on level 10 (after all monsters dead): transition to `GameState::Victory`.

---

## Dungeon Generation

**Map size:** 90 wide × 40 tall tiles.

**Algorithm:** Rooms-and-corridors

1. Attempt to place 6–10 rectangular rooms of random size (min 4×4, max 10×8), rejecting overlapping rooms
2. Connect each room to the previous with an L-shaped corridor (horizontal then vertical)
3. Place the player `@` in the center of the first room
4. Spawn exactly one Goblin, one Mage, and one Rat such that:
   - Each monster occupies a different room
   - No monster spawns in the player's starting room
   - Each monster is placed at a random floor tile within its assigned room
   - If fewer than 4 rooms exist (very unlikely with 6–10 room generation), assign each monster to a room — two may share a room, but each must spawn at a distinct floor tile that is not the player's starting position
5. Scatter 0–2 potions `P` in random floor tiles (~30% chance each potion spawns), not on monster or player tiles
6. Staircase `>` is not placed at generation time — it spawns dynamically when all monsters are dead (see Level Progression)

**Tile types:**

```rust
enum Tile {
    Wall,
    Floor,
    Corridor,
    Stair,
}
```

**Tile rendering:**
| Tile     | Char | Color          |
|----------|------|----------------|
| Wall     | `#`  | Dark grey      |
| Floor    | `.`  | Very dark grey |
| Corridor | `.`  | Very dark grey |
| Stair    | `>`  | Yellow         |

---

## Entity System

### Player

```rust
struct Player {
    x: u16,
    y: u16,
    hp: i32,
    max_hp: i32,   // starts at 20
}
```

- Starts with `hp: 20, max_hp: 20`
- Defeating a monster grants `+1 max_hp` and `+1 hp`
- Movement is cardinal only (N/S/E/W) — no diagonal movement

### Monster

```rust
enum MonsterKind { Goblin, Mage, Rat }

struct Monster {
    x: u16,
    y: u16,
    kind: MonsterKind,
    hp: i32,
    alive: bool,
}
```

**Monster stats:**
| Monster | Char | Color               | Starting HP |
|---------|------|---------------------|-------------|
| Goblin  | `G`  | Blue                | 8           |
| Mage    | `M`  | Red                 | 12          |
| Rat     | `R`  | Dark Yellow (brown) | 5           |

**Note:** Ratatui has no native brown. `Color::Yellow` with dark styling renders as a brownish tone in most terminals.

### Movement

**Player:** Arrow keys move one tile. If the destination tile contains a living monster, combat triggers instead of movement (player does not move). If the destination tile contains a potion, the potion is consumed and the player moves onto the tile. If a tile contains both a monster and a potion (possible at dungeon generation), monster combat takes priority; the potion is not consumed until the monster is dead and the player steps on the tile again. There is no "wait" action — turns only advance on player movement.

**Monsters:** Each turn, every living monster moves one step closer to the player:
1. Calculate `|monster.x - player.x|` (dx) and `|monster.y - player.y|` (dy)
2. Move along the axis with greater distance
3. If dx == dy (tie), prefer horizontal movement (x-axis)
4. Skip movement if the destination is a wall or occupied by another monster
5. Monsters cannot consume potions; they move through potion tiles without effect

---

## Combat

### Tile Interaction Priority

When the player moves to a tile, resolve in this order:
1. If tile contains a living monster → trigger combat (player stays in place, monster stays in place)
2. If tile contains a potion → consume potion, player moves to tile
3. If tile is a staircase → descend (or win if level 10)
4. If tile is walkable floor/corridor → player moves normally

### Player Attacks Monster

1. Roll 1d6 — on 4, 5, or 6: hit
2. If hit, roll 1d6 damage, subtract from monster HP
3. If monster HP ≤ 0: monster dies (`alive = false`), player gains `+1 max_hp` and `+1 hp`

### Monster Counter-attacks (same turn, only if monster still alive)

1. Roll 1d6 — on 4, 5, or 6: hit
2. If hit, roll 1d6 damage, subtract from player HP
3. If player HP ≤ 0: immediately set `GameState::GameOver`, stop all further processing

### Monster Moves Onto Player

When a monster's movement lands it on the player's tile during step 4 of the turn order:
1. Roll 1d6 — on 4, 5, or 6: hit
2. If hit, roll 1d6 damage, subtract from player HP
3. If player HP ≤ 0: immediately set `GameState::GameOver`, stop processing remaining monsters

### Potions

Stepping on a `P` tile: set `hp = max_hp` (full heal), convert tile to `Floor`. Monsters cannot consume potions.

### Rendering Priority (same tile)

In the unlikely event of overlapping entities, render in this priority order (highest first):
1. Player `@`
2. Monsters `G`, `M`, `R`
3. Potions `P`
4. Stairs `>`
5. Tile background (wall, floor, corridor)

### Status Bar Messages

Single line updated after each player action. Examples:
- `"You hit the Goblin for 3 damage! (5 HP left)"`
- `"You missed the Rat!"`
- `"The Mage hits you for 4 damage! (14 HP left)"`
- `"The Goblin missed you!"`
- `"You slew the Mage! +1 max HP"`
- `"You drank a potion and feel restored! (20 HP)"`
- `"The way down is revealed..."`
- `"The stairs lead deeper into the dungeon..."`

---

## Rendering

### Layout

**Column allocation:** 90 (dungeon) + 1 (border) + 19 (sidebar) = 110 columns total. Minimum terminal width is 110; minimum terminal height is 44 (1 top bar + 40 dungeon + 1 border + 1 status bar + 1 padding).

```
┌────────────────────────────────────────────────────────────────────────────────────────────┐
│                     ⚔ Return to Dagger Deep ⚔                       Level: 3/10            │  ← top bar (1 line)
├────────────────────────────────────────────────────────────────────────────────┬───────────┤
│                                                                                │  PLAYER   │
│   90×40 dungeon map                                                            │  HP: 18/20│
│                                                                                │           │
│                                                                                │  LEGEND   │
│                                                                                │  @ You    │
│                                                                                │  G Goblin │
│                                                                                │  M Mage   │
│                                                                                │  R Rat    │
│                                                                                │  P Potion │
│                                                                                │  > Stairs │
├────────────────────────────────────────────────────────────────────────────────┴───────────┤
│  You hit the Goblin for 3 damage! (5 HP left)                                              │  ← status bar (1 line)
└────────────────────────────────────────────────────────────────────────────────────────────┘
```

**Sidebar width:** 19 characters (including the border).

### Widget Details

- **Top bar:** `Paragraph`, centered, cyan text on dark blue background, shows title and current level
- **Dungeon pane:** Rendered row-by-row as `Line` spans, each cell individually colored
- **Sidebar:** `Paragraph`, HP shown in red when below 50%, green otherwise; legend in fixed character colors
- **Status bar:** `Paragraph`, yellow text, single line updated after each player action
- **Game Over screen:** Full-screen centered block replacing dungeon pane, red title, prompt to press any key to exit
- **Victory screen:** Full-screen centered block, gold/yellow title, congratulations message, prompt to press any key to exit

---

## Input Handling

| Key          | State           | Action                        |
|--------------|-----------------|-------------------------------|
| Arrow Up     | Playing         | Move player north             |
| Arrow Down   | Playing         | Move player south             |
| Arrow Left   | Playing         | Move player west              |
| Arrow Right  | Playing         | Move player east              |
| Escape       | Playing         | Quit game immediately         |
| Any key      | GameOver        | Exit process                  |
| Any key      | Victory         | Exit process                  |

---

## Win / Lose Conditions

- **Win:** Player clears all 3 monsters on level 10 and steps on `>`
- **Lose:** Player HP reaches 0 at any point (during player-initiated combat or monster attack)
- **Quit:** Player presses Escape during `Playing` state
