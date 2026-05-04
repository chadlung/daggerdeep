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
    pub status_msg2: String,
    pub state: u8,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct NetPlayer {
    pub x: usize,
    pub y: usize,
    pub hp: i32,
    pub max_hp: i32,
    pub respawns_left: u8,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct NetMonster {
    pub x: usize,
    pub y: usize,
    pub kind: u8,
    pub hp: i32,
    pub max_hp: i32,
    pub alive: bool,
    pub teleported: bool,
}
