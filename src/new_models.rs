use serde::{Deserialize, Serialize};
use crate::schema::{game, turn, player, mutation, card_options_table};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub struct Player {
    pub id: i32,
    pub private_key: String,
    pub username: String,
    pub character_type: i32,
    pub pos_x: i32,
    pub pos_y: i32,
    pub pos_orientation: i32,
    pub is_ai: bool,
    pub game_id: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub struct Game {
    pub id: u64,
    pub board_size: (u64, u64),
    pub game_started: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub struct Turn {
    pub id: i32,
    pub game_id: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NewPlayer {
    pub username: String,
    pub character_type: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub struct Mutation {
    pub id: i32,
    pub card_type: i32,
    pub card_location: i32,
    pub turn_id: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub struct CardOptions {
    pub id: i32,
    pub card_options: Vec<i32>,
    pub player_id: i32,
    pub turn_id: i32,
}
