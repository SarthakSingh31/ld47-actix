use std::time::{Duration, Instant};
use std::collections::{HashMap, HashSet};
use actix::prelude::*;
use serde::Serialize;
use rand::prelude::*;
use rand::Rng;
use crate::models::{Game, Player, Mutation, CardOptions};

const NUMBER_OF_CARDS: u8 = 5;
const MAX_PLAYERS: usize = 10;
const BOARD_SIZE: (u16, u16) = (16, 9);

#[derive(Message)]
#[rtype(result = "()")]
pub struct ToUserMessage(pub String);

#[derive(Message)]
#[rtype(usize)]
pub struct Connect {
    pub username: String,
    pub character_type: u8,
    pub addr: Option<Recipient<ToUserMessage>>,
}

#[derive(Message)]
#[rtype(usize)]
pub struct MutationMessage {
    pub mutation: Mutation,
    pub player_id: usize,
    pub pk: String,
    pub turn_id: usize,
    pub game_id: usize,
}

#[derive(Message)]
#[rtype(usize)]
pub struct CountDownMessage {
    pub game_id: usize,
}

#[derive(Message)]
#[rtype(usize)]
pub struct CreateTurnMessage {
    pub game_id: usize,
    pub check_turn_id: Option<usize>,
}

#[derive(Message)]
#[rtype(usize)]
pub struct CardChoiceMessage {
    pub game_id: usize,
    pub turn_id: usize,
}

/// Session is disconnected
#[derive(Message)]
#[rtype(i32)]
pub struct Disconnect {
    pub id: usize,
}

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "type")]
pub struct PlayerJoin {
    pub user_id: usize,
    pub username: String,
    pub x: u16,
    pub y: u16,
    pub char_type: u8,
    pub start_orientation: u8,
}

#[derive(Message)]
#[rtype(result = "()")]
pub struct BroadcastObj<O: Serialize> {
    pub serializabe_obj: O,
    pub ori_player_id: Option<usize>,
    pub player_addrs_and_ids: Vec<(usize, Recipient<ToUserMessage>)>
}

#[derive(Message)]
#[rtype(result = "()")]
pub struct BroadcastStr {
    pub json_string: String,
    pub ori_player_id: Option<usize>,
    pub player_addrs_and_ids: Vec<(usize, Recipient<ToUserMessage>)>
}

#[derive(Message)]
#[rtype(usize)]
pub struct AnimationMessage {
    pub player_id: usize,
    pub pk: String,
    pub game_id: usize,
    pub turn_id: usize,
}

#[derive(Message)]
#[rtype(usize)]
pub struct PollPlayerDeathMessage {
    pub other_id: usize,
    pub self_id: usize,
    pub pk: String,
    pub game_id: usize,
    pub turn_id: usize,
}

impl PlayerJoin {
    fn from(player: &Player, index: usize) -> Self {
        PlayerJoin {
            user_id: index,
            username: player.username.clone(),
            x: player.pos.0,
            y: player.pos.1,
            char_type: player.character_type,
            start_orientation: player.pos.2,
        }
    }
}

pub struct GameServer {
    games: HashMap<usize, Game>,
    rng: ThreadRng,
}

impl GameServer {
    pub fn new() -> GameServer {
        GameServer {
            games: HashMap::new(),
            rng: rand::thread_rng(),
        }
    }

    fn broadcast_to_game<O: Serialize>(
        serializabe_obj: &O,
        ori_player_index: Option<usize>,
        player_addrs_and_ids: Vec<(usize, Recipient<ToUserMessage>)>
    ) {
        if let Ok(json_string) = serde_json::to_string(serializabe_obj) {
            GameServer::broadcast_to_game_str(json_string, ori_player_index, player_addrs_and_ids);
        }
    }

    fn broadcast_to_game_str(
        json_string: String,
        ori_player_index: Option<usize>,
        player_addrs_and_ids: Vec<(usize, Recipient<ToUserMessage>)>
    ) {
        for (i, addr) in player_addrs_and_ids.iter() {
            if let Some(ori_player_index) = ori_player_index {
                if ori_player_index != *i {
                    let _ = addr.do_send(ToUserMessage(json_string.clone()));
                }
            } else {
                let _ = addr.do_send(ToUserMessage(json_string.clone()));
            }
        }
    }

    fn fill_slots_with_ai(game: &mut Game, rng: &mut ThreadRng, ctx: &mut Context<Self>) {
        for n in 0 .. (MAX_PLAYERS - game.players.len()) {
            ctx.address().do_send(Connect {
                username: format!("Bot {}", n),
                character_type: rng.gen_range(0, NUMBER_OF_CARDS),
                addr: None,
            });
        }
    }

    fn get_real_active_players(game: &mut Game) -> Vec<&Player> {
        game.players.iter().filter(|p| !p.is_ai && p.active).collect()
    }
}

/// Make actor from `GameServer`
impl Actor for GameServer {
    /// We are going to use simple Context, we just need ability to communicate
    /// with other actors.
    type Context = Context<Self>;
}

impl Handler<Connect> for GameServer {
    type Result = usize;

    fn handle(&mut self, connect: Connect, ctx: &mut Context<Self>) -> Self::Result {
        let current_game: &mut Game;
        let mut key: usize;
        let mut open_games: Vec<(_, &mut Game)> = self.games.iter_mut().filter(|(_, game)| game.game_started).collect();

        if open_games.len() > 0 {
            key = *open_games[0].0;
            current_game = open_games[0].1;
        } else {
            key = self.rng.gen();
            while self.games.contains_key(&key) { key = self.rng.gen(); }

            let mut available_pos = HashSet::new();
            for _ in 0 .. MAX_PLAYERS {
                let mut new_pos = (
                    self.rng.gen_range(0, BOARD_SIZE.0),
                    self.rng.gen_range(1, BOARD_SIZE.1)
                );

                while available_pos.contains(&new_pos) {
                    new_pos = (
                        self.rng.gen_range(0, BOARD_SIZE.0),
                        self.rng.gen_range(1, BOARD_SIZE.1)
                    );
                }

                available_pos.insert(new_pos);
            }

            self.games.insert(key, Game {
                board_size: BOARD_SIZE,
                game_started: false,
                players: Vec::new(),
                turn_index: 0,
                available_pos: available_pos.into_iter().collect(),
                game_countdown_handle: None,
                has_loop_countdown: false,
            });
            current_game = self.games.get_mut(&key).unwrap();

            ctx.address().do_send(CountDownMessage {
                game_id: key,
            });            
        }

        println!("{} joined", connect.username);
        if !current_game.available_pos.is_empty() {
            let pos = current_game.available_pos.remove(0);
            let player_id = current_game.players.len();
            current_game.players.push(Player::new(
                player_id,
                key,
                connect.username,
                connect.character_type,
                (pos.0, pos.1),
                connect.addr.is_none(),
                connect.addr.clone(),
                &mut self.rng,
            ));
            let current_player = current_game.players.last().unwrap();
            if let Some(ref addr) = connect.addr {
                if let Ok(json_string) = serde_json::to_string(&current_player) {
                    let _ = addr.do_send(ToUserMessage(json_string));
                }
            }

            for some_player in  current_game.players.iter().enumerate().map(|(i, p)| PlayerJoin::from(p, i)) {
                if let Ok(json_string) = serde_json::to_string(&some_player) {
                    if let Some(ref addr) = connect.addr {
                        let _ = addr.do_send(ToUserMessage(json_string));
                    }
                }
            }
            Self::broadcast_to_game(
                &PlayerJoin::from(current_player, player_id),
                Some(player_id),
                current_game.get_cloned_players_id_addr()
            );

            // if max players has reached start game
            if current_game.players.len() == MAX_PLAYERS && !current_game.game_started {
                ctx.address().do_send(CreateTurnMessage {
                    game_id: key,
                    check_turn_id: None,
                });
            }

            if current_player.is_ai {
                ctx.address().do_send(MutationMessage {
                    mutation: Mutation {
                        card_type: self.rng.gen_range(0, NUMBER_OF_CARDS),
                        card_location: 0,
                    },
                    player_id: current_player.id,
                    pk: current_player.private_key.clone(),
                    game_id: current_player.game_id,
                    turn_id: current_game.turn_index,
                });
            }

            player_id
        } else {
            if let Some(addr) = connect.addr {
                let _ = addr.do_send(ToUserMessage(String::from("Game already full")));
            }
            0
        }
    }
}

impl Handler<MutationMessage> for GameServer {
    type Result = usize;

    fn handle(&mut self, mutation: MutationMessage, _: &mut Context<Self>) -> Self::Result {
        let game = self.games.get_mut(&mutation.game_id).unwrap();
        let addr_data = game.get_cloned_players_id_addr();
        if game.players[mutation.player_id].private_key == mutation.pk {
            game.players[mutation.player_id].previous_choices.push(mutation.mutation);
            Self::broadcast_to_game(
                game.players[mutation.player_id].previous_choices.last().unwrap(),
                None,
                addr_data
            );
        } else {
            if let Some(ref addr) = game.players[mutation.player_id].addr {
                let _ = addr.do_send(ToUserMessage(String::from("Wrong pk")));
            }
        }
        mutation.player_id
    }
}

impl Handler<CountDownMessage> for GameServer {
    type Result = usize;

    fn handle(&mut self, countdown: CountDownMessage, ctx: &mut Context<Self>) -> Self::Result {
        let start = Instant::now();
        let game_id = countdown.game_id;
        let game = self.games.get_mut(&countdown.game_id).unwrap();
        let addr_data = game.get_cloned_players_id_addr();
        game.game_countdown_handle = Some(ctx.run_interval(Duration::from_secs(1), move |_act, ctx| {
            let secs = Instant::now().duration_since(start);
            if secs <= Duration::from_secs(10) {
                ctx.address().do_send(BroadcastStr {
                    json_string: format!("{{\"type\": \"TillStart\", \"secs\": \"{}\"}}", 10 - secs.as_secs()),
                    ori_player_id: None,
                    player_addrs_and_ids: addr_data.clone()
                });
            } else {
                ctx.address().do_send(CreateTurnMessage {
                    game_id: game_id,
                    check_turn_id: None,
                });
            }
        }));

        countdown.game_id
    }
}

impl<O: Serialize> Handler<BroadcastObj<O>> for GameServer {
    type Result = ();

    fn handle(&mut self, obj: BroadcastObj<O>, _: &mut Context<Self>) {
        GameServer::broadcast_to_game(&obj.serializabe_obj, obj.ori_player_id, obj.player_addrs_and_ids);
    }
}

impl Handler<BroadcastStr> for GameServer {
    type Result = ();

    fn handle(&mut self, obj: BroadcastStr, _: &mut Context<Self>) {
        GameServer::broadcast_to_game_str(obj.json_string, obj.ori_player_id, obj.player_addrs_and_ids);
    }
}

impl Handler<CreateTurnMessage> for GameServer {
    type Result = usize;

    fn handle(&mut self, gameinfo: CreateTurnMessage, ctx: &mut Context<Self>) -> Self::Result {
        let mut current_game = self.games.get_mut(&gameinfo.game_id).unwrap();
        let mut rng = self.rng;

        if let Some(check_turn_id) = gameinfo.check_turn_id {
            current_game.has_loop_countdown = false;
            if check_turn_id != current_game.turn_index { return 0; }
        }

        if let Some(countdown_handle) = current_game.game_countdown_handle {
            ctx.cancel_future(countdown_handle);
            current_game.game_countdown_handle = None;
            current_game.game_started = true;

            GameServer::fill_slots_with_ai(current_game, &mut rng, ctx);
        } else {
            let players_with_no_move = current_game.players.iter().filter(|p| p.previous_choices.len() < current_game.turn_index);

            for player in players_with_no_move {
                ctx.address().do_send(MutationMessage {
                    mutation: Mutation {
                        card_type: self.rng.gen_range(0, NUMBER_OF_CARDS),
                        card_location: 0,
                    },
                    player_id: player.id,
                    pk: player.private_key.clone(),
                    game_id: player.game_id,
                    turn_id: current_game.turn_index,
                });
            }

            for player in current_game.players.iter_mut() {
                player.animation_done = false;
                player.has_death_voted = false;
                player.been_death_voted_for = 0;
            }

            current_game.turn_index += 1;
        }
        
        ctx.address().do_send(CardChoiceMessage {
            game_id: gameinfo.game_id,
            turn_id: current_game.turn_index,
        });

        for ai_player in current_game.players.iter().filter(|p| p.is_ai) {
            ctx.address().do_send(MutationMessage {
                mutation: Mutation {
                    card_type: self.rng.gen_range(0, NUMBER_OF_CARDS),
                    card_location: 0,
                },
                player_id: ai_player.id,
                pk: ai_player.private_key.clone(),
                game_id: ai_player.game_id,
                turn_id: current_game.turn_index,
            });
        }

        gameinfo.game_id
    }
}

impl Handler<CardChoiceMessage> for GameServer {
    type Result = usize;

    fn handle(&mut self, gameinfo: CardChoiceMessage, _: &mut Context<Self>) -> Self::Result {
        let current_game = self.games.get_mut(&gameinfo.game_id).unwrap();
        for (i, player) in  current_game.players.iter_mut().enumerate() {
            player.card_options = Some(vec![
                self.rng.gen_range(0, NUMBER_OF_CARDS),
                self.rng.gen_range(0, NUMBER_OF_CARDS),
                self.rng.gen_range(0, NUMBER_OF_CARDS),
            ]); 

            let card_options = CardOptions {
                card_options: player.card_options.as_ref().unwrap().clone(),
                player_id: i,
                turn_id: gameinfo.turn_id,
            };
            if let Ok(json_string) = serde_json::to_string(&card_options) {
                if let Some(ref addr) = player.addr {
                    let _ = addr.do_send(ToUserMessage(json_string));
                }
            }
        }
        gameinfo.turn_id
    }
}

impl Handler<AnimationMessage> for GameServer {
    type Result = usize;

    fn handle(&mut self, gameinfo: AnimationMessage, ctx: &mut Context<Self>) -> Self::Result {
        let current_game = self.games.get_mut(&gameinfo.game_id).unwrap();
        current_game.players[gameinfo.player_id].animation_done = true;

        let players_with_animation_count = current_game
            .players
            .iter()
            .filter(|p| p.animation_done)
            .collect::<Vec<_>>().len();

        let game_id = gameinfo.game_id;
        let active_player_count = Self::get_real_active_players(current_game).len();

        if players_with_animation_count == MAX_PLAYERS - active_player_count {
            ctx.address().do_send(CreateTurnMessage {
                game_id: game_id,
                check_turn_id: None,
            });
        } else if players_with_animation_count as f64 >= (active_player_count as f64 * 0.9f64) && !current_game.has_loop_countdown {
            current_game.has_loop_countdown = true;
            let current_turn_index = current_game.turn_index;
            ctx.run_later(Duration::from_secs(10), move |_act, ctx| {
                ctx.address().do_send(CreateTurnMessage {
                    game_id: game_id,
                    check_turn_id: Some(current_turn_index),
                });
            });
        }

        gameinfo.player_id
    }
}

impl Handler<PollPlayerDeathMessage> for GameServer {
    type Result = usize;

    fn handle(&mut self, gameinfo: PollPlayerDeathMessage, ctx: &mut Context<Self>) -> Self::Result {
        let current_game = self.games.get_mut(&gameinfo.game_id).unwrap();

        if !current_game.players[gameinfo.self_id].has_death_voted && gameinfo.pk == current_game.players[gameinfo.self_id].private_key && current_game.players[gameinfo.other_id].active {
            current_game.players[gameinfo.self_id].has_death_voted = true;
            current_game.players[gameinfo.other_id].been_death_voted_for += 1;

            let active_player_count = Self::get_real_active_players(current_game).len();

            if current_game.players[gameinfo.other_id].been_death_voted_for as f64 >= active_player_count as f64 * 0.5f64 {
                current_game.players[gameinfo.other_id].active = false;
            }
        }

        gameinfo.self_id
    }
} 

impl Handler<Disconnect> for GameServer {
    type Result = i32;

    fn handle(&mut self, msg: Disconnect, _: &mut Context<Self>) -> Self::Result {
        println!("Someone left");
        0
    }
}
