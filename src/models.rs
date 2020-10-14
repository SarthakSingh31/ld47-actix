use serde::Serialize;
use actix::prelude::*;
use rand::prelude::*;
use rand::distributions::Alphanumeric;

use crate::server::ToUserMessage;

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "type")]
pub struct Mutation {
    pub user_id: usize,
    pub card_type: u8,
    pub card_location: u8,
}

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "type")]
pub struct Player {
    pub id: usize,
    pub username: String,
    pub private_key: String,
    pub character_type: u8,
    pub pos: (u16, u16, u8),
    pub is_ai: bool,
    pub active: bool,
    #[serde(skip)]
    pub previous_choices: Vec<Mutation>,
    #[serde(skip)]
    pub card_options: Option<Vec<u8>>,
    #[serde(skip)]
    pub animation_done: bool,
    #[serde(skip)]
    pub addr: Option<Recipient<ToUserMessage>>,
    pub game_id: usize,
    #[serde(skip)]
    pub has_death_voted: bool,
    #[serde(skip)]
    pub been_death_voted_for: usize,
}

impl Player {
    pub fn new(
        id: usize,
        game_id: usize,
        username: String,
        character_type: u8,
        pos: (u16, u16),
        is_ai: bool,
        addr: Option<Recipient<ToUserMessage>>,
        rng: &mut ThreadRng
    ) -> Player {
        return Player {
            id: id,
            username: username,
            private_key: rng.sample_iter(&Alphanumeric).take(10).collect::<String>(),
            character_type: character_type,
            pos: (pos.0, pos.1, rng.gen_range(0, 4)),
            is_ai: is_ai,
            active: true,
            previous_choices: Vec::new(),
            card_options: None,
            animation_done: false,
            addr: addr,
            game_id: game_id,
            has_death_voted: false,
            been_death_voted_for: 0,
        }
    }
}

pub struct Game {
    pub board_size: (u16, u16),
    pub game_started: bool,
    pub players: Vec<Player>,
    pub turn_index: usize,
    pub available_pos: Vec<(u16, u16)>,
    pub game_countdown_handle: Option<SpawnHandle>,
    pub has_loop_countdown: bool,
    pub game_tick_handle: SpawnHandle,
}

impl Game {
    pub fn get_cloned_players_id_addr(&self) -> Vec<(usize, Recipient<ToUserMessage>)> {
        let mut ret = Vec::new();

        for (i, player) in self.players.iter().enumerate() {
            if let Some(ref addr) = player.addr {
                ret.push((i, addr.clone()));
            }
        }

        ret
    }
}

// Helper structs

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "type")]
pub struct Turn {
    pub id: i32,
}

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "type")]
pub struct CardOptions {
    pub card_options: Vec<u8>,
    pub player_id: usize,
    pub turn_id: usize,
}
