use serde::{Deserialize, Serialize};
use crate::schema::{game, turn, player, mutation, card_options_table};

#[derive(Debug, Clone, Serialize, Deserialize, Queryable, Insertable, Identifiable)]
#[table_name = "game"]
pub struct Game {
    pub id: i32,
    pub game_started: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, Queryable, Insertable, Identifiable, Associations)]
#[belongs_to(Game)]
#[table_name = "turn"]
pub struct Turn {
    pub id: i32,
    pub game_id: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize, Queryable, Insertable, Identifiable, Associations)]
#[belongs_to(Game)]
#[table_name = "player"]
pub struct Player {
    pub id: i32,
    pub private_key: String,
    pub username: String,
    pub character_type: i32,
    pub is_ai: bool,
    pub game_id: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NewPlayer {
    pub username: String,
    pub character_type: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize, Queryable, Insertable, Identifiable, Associations)]
#[belongs_to(Turn)]
#[table_name = "mutation"]
pub struct Mutation {
    pub id: i32,
    pub card_type: i32,
    pub card_location: i32,
    pub turn_id: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize, Queryable, Insertable, Identifiable, Associations)]
#[belongs_to(Player)]
#[belongs_to(Turn)]
#[table_name = "card_options_table"]
pub struct CardOptions {
    pub id: i32,
    pub card_options: Vec<i32>,
    pub player_id: i32,
    pub turn_id: i32,
}