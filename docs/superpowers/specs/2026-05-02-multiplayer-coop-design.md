# Return to Dagger Deep — Multiplayer Co-op Design Spec

**Date:** 2026-05-02
**Scope:** Add two-player cooperative multiplayer via TCP, chest mechanic, and scaled difficulty.

---

## Overview

Two players share one dungeon in a cooperative run. One player hosts (authoritative server); the other joins as a client. The host owns all game state and sends full snapshots after every resolved turn. The client is a display terminal with a keyboard — it holds no game logic.

Single-player mode is unchanged. All new code paths are gated behind `Game::multiplayer: bool`.

---

## Architecture

### Threading Model

**Host:**
```
main thread:  crossterm poll → resolve P1 move → drain P2 channel → resolve P2 move
            → move_monsters → serialize NetGameState → send to client → render
recv thread:  TcpStream::read → deserialize ClientMsg → mpsc::Sender<ClientMsg>
```

**Client:**
```
main thread:  crossterm poll → send ClientMsg → drain server channel → render NetGameState
recv thread:  TcpStream::read → deserialize ServerMsg → mpsc::Sender<ServerMsg>
```

### New Modules

| File | Responsibility |
|------|---------------|
| `src/protocol.rs` | `ClientMsg`, `ServerMsg`, `NetGameState`, `NetPlayer`, `NetMonster` — all `serde`-derived |
| `src/net.rs` | `send_msg` / `recv_msg` with `u32` length-prefix framing; channel wiring helpers |
| `src/menu.rs` | TUI start screen: single-player / host / join + IP input |

### New Dependencies

```toml
serde = { version = "1", features = ["derive"] }
bincode = "1"
```

No tokio. No async. `std::net::TcpListener` / `TcpStream` + `std::sync::mpsc`.

---

## Network Protocol

### Framing

Every message is length-prefixed: a `u32` (4 bytes, little-endian) written first, followed by that many bytes of `bincode`-serialized payload. Handles TCP stream boundaries with no third-party framing crate.

### Message Types

```rust
// client → host
enum ClientMsg {
    Move(i32, i32),  // dx, dy
    Quit,
}

// host → client
enum ServerMsg {
    State(NetGameState),
    Disconnected,
}
```

### NetGameState

Sent as a full snapshot every turn (~4 KB max). No deltas, no sequence numbers.

```rust
struct NetGameState {
    map_tiles: Vec<Vec<u8>>,      // Tile discriminant as u8
    player1: NetPlayer,
    player2: Option<NetPlayer>,   // None when host continues solo after P2 disconnect
    monsters: Vec<NetMonster>,
    level: u8,
    status_msg: String,
    state: u8,                    // GameState discriminant as u8
}

struct NetPlayer {
    x: usize,
    y: usize,
    hp: i32,
    max_hp: i32,
    alive: bool,
}

struct NetMonster {
    x: usize,
    y: usize,
    kind: u8,   // MonsterKind discriminant
    hp: i32,
    alive: bool,
}
```

---

## Game State Changes

### Player 2

- `Game` gains `pub player2: Option<Player>` — `None` in single-player.
- `Game` gains `pub multiplayer: bool`.
- P2 spawns in `map.rooms[0].center()` offset by `(1, 0)` from P1.
- P2 spawns at `(center.x + 1, center.y)` — always one tile to the right of P1.
- P2 on-screen character: `&` (cyan).
- Sidebar gains a second HP bar when `player2.is_some()`.

### Respawn

When P2's HP ≤ 0:
- Respawn position: `map.rooms[0].center()` offset by `(1, 0)`.
- Respawn HP: `player2.max_hp / 2`.
- Status message: `"P2 was defeated and respawned!"`.

### Stairs Sync

When either player steps on `Tile::Stair`, `next_level()` repositions **both** players to room 0 center (P1 at center, P2 at center + 1 tile). No player is left behind.

### Chest Tile

`Tile::Chest` added to the `Tile` enum in `dungeon.rs`. Rendered as `$` (yellow).

**Spawn:** 1–2 chests per level, placed on walkable tiles not occupied by monsters or potions.

**`open_chest(opener_is_p2: bool)` in `game.rs`:**

| RNG roll (f32) | Outcome | Effect |
|----------------|---------|--------|
| < 0.40 | Food | Both players restored to `max_hp`; tile → `Floor` |
| < 0.65 | Trap | Opener takes `roll_d6()` damage; tile → `Floor` |
| else | Empty | Status: `"The chest was empty."`; tile → `Floor` |

### Difficulty Scaling (multiplayer only)

Applied only when `multiplayer == true`:

- **Monster HP:** `kind.starting_hp()` × 1.5, rounded up. Applied at spawn in `spawn_monsters`.
- **Monster attack:** `resolve_monster_attack` rolls two d6 and uses the higher value (advantage) when `multiplayer == true`.

`spawn_monsters(map, rng, multiplayer: bool)` and `resolve_monster_attack(..., multiplayer: bool)` receive the flag explicitly.

---

## Turn Flow

### Host Game Loop (multiplayer)

```
1. Render current state
2. event::poll(50ms timeout)
3. If P1 key received: resolve move_player(&mut player1, dx, dy)
4. Drain mpsc channel:
     - ClientMsg::Move(dx, dy) → resolve move_player(&mut player2, dx, dy)
     - ClientMsg::Quit → remove P2, continue solo
5. move_monsters()
6. Serialize NetGameState → send to client via net::send_msg
7. goto 1
```

P1 never waits for P2. If no `ClientMsg::Move` arrives this frame, P2 simply skips their turn.

### Client Game Loop

```
1. Render last received NetGameState
2. event::poll(50ms timeout)
3. If key: net::send_msg(ClientMsg::Move(dx, dy)) to host
4. Drain mpsc channel:
     - ServerMsg::State(s) → update local display state
     - ServerMsg::Disconnected → set state = Disconnected, show overlay
5. goto 1
```

Client calls no game logic. All resolution happens on the host.

---

## TUI Menu

Three screens rendered in `menu.rs` before the game loop starts:

**Main menu:**
```
[S] Single Player
[H] Host Game
[J] Join Game
[Q] Quit
```

**Host screen:**
- Displays: `"Waiting for player 2 on port 4444..."`
- Animated spinner while blocking on `TcpListener::accept()`
- Port hardcoded: `4444`
- On connect: transitions to game loop

**Join screen:**
- Text input for IP address (e.g. `192.168.1.5`)
- Port hardcoded: `4444`
- `Enter` connects; `Esc` returns to main menu
- Shows `"Connecting..."` then enters game, or shows inline error on failure

---

## Error Handling & Disconnection

Network errors are **fatal** — no reconnect logic.

`GameState` gains a `Disconnected` variant (alongside `Playing`, `GameOver`, `Victory`, `ConfirmQuit`).

| Event | Behaviour |
|-------|-----------|
| Client recv error / EOF | Client sets `state = Disconnected`, renders `"Connection lost. Press any key to exit."` |
| Host recv error for client stream | Host sets `player2 = None`, posts `"P2 disconnected. Continuing solo."` |
| `ClientMsg::Quit` received by host | Host sets `player2 = None`, posts `"P2 quit. Continuing solo."` |
| Serialization error | Treated as connection error; log to stderr, disconnect |

---

## File Change Summary

| File | Change |
|------|--------|
| `Cargo.toml` | Add `serde`, `bincode` |
| `src/protocol.rs` | New — message types and net state structs |
| `src/net.rs` | New — framed send/recv, channel wiring |
| `src/menu.rs` | New — TUI start screen |
| `src/dungeon.rs` | Add `Tile::Chest`; update `is_walkable`; add chest spawn to `generate` |
| `src/entity.rs` | No changes |
| `src/combat.rs` | `resolve_monster_attack` gains `multiplayer: bool`; rolls with advantage when true |
| `src/game.rs` | Add `player2`, `multiplayer`; update `new`, `next_level`, `move_player`, `move_monsters`; add `open_chest`, `spawn_monsters` multiplayer flag |
| `src/render.rs` | Render `&` for P2; render `$` for chest; P2 HP bar in sidebar; disconnection overlay |
| `src/main.rs` | Launch menu before game loop; wire host/client network paths |
