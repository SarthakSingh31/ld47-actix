use std::time::{Duration, Instant};
use std::collections::{HashMap, HashSet};
use actix::prelude::*;
use serde::Serialize;
use rand::prelude::*;
use rand::Rng;
use rand::distributions::WeightedIndex;
use crate::models::{Game, Player, Mutation, CardOptions};
use crate::config::{Card, Animation, GameConfig};

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
    pub game_id: Option<usize>, // Only for bot use
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

#[derive(Message)]
#[rtype(usize)]
pub struct MessagePrune {
    pub addr: Recipient<ToUserMessage>,
}

#[derive(Message)]
#[rtype(usize)]
pub struct MessageFullClean {
    pub addr: Recipient<ToUserMessage>,
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
    pub game_id: usize,
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
pub struct GameTickerMessage {
    pub game_id: usize,
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

#[derive(Message)]
#[rtype(usize)]
pub struct DeleteGame {
    pub game_id: usize,
    pub game_tick_handle: SpawnHandle,
}

impl PlayerJoin {
    fn from(player: &Player) -> Self {
        PlayerJoin {
            user_id: player.id,
            username: player.username.clone(),
            x: player.pos.0,
            y: player.pos.1,
            char_type: player.character_type,
            start_orientation: player.pos.2,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "type")]
pub struct PlayerDied {
    pub user_id: usize,
}

pub struct GameServer {
    games: HashMap<usize, Game>,
    rng: ThreadRng,
    cardDist: WeightedIndex<f64>,
    gc: GameConfig,
}

impl GameServer {
    pub fn new() -> GameServer {
        GameServer {
            games: HashMap::new(),
            rng: rand::thread_rng(),
            cardDist: WeightedIndex::new(vec![15f64,
                15f64,
                5f64,
                10f64,
                15f64,
                15f64,
                5f64,
                0f64,
                0f64,
                3.6f64,
                3.6f64,
                3.6f64,
                3.6f64,
                3.6f64,
                3.6f64,
                3.6f64,
                2.5f64,
                2.5f64,
                3.6f64,
                3.6f64,
                3.6f64,
                3.6f64,
                2.3f64,
                2.3f64,
                2.3f64,
                2.3f64,
                3.6f64,
                3.6f64,
                2.0f64,
                2.0f64,
                2.0f64,
                2.0f64,
                2.0f64,
                1.2f64,
                2.0f64,
                2.0f64,
                2.0f64,
                1.2f64]).unwrap(),
            gc: GameConfig::default(),
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

    fn fill_slots_with_ai(game: &mut Game, rng: &mut ThreadRng, ctx: &mut Context<Self>, game_id: usize, gc: &GameConfig) {
        for n in 0 .. (MAX_PLAYERS - game.players.len()) {
            ctx.address().do_send(Connect {
                username: format!("Bot {}", n),
                character_type: rng.gen_range(0, 3),
                addr: None,
                game_id: Some(game_id),
            });
        }
    }

    fn get_real_active_players(game: &Game) -> Vec<&Player> {
        game.players.iter().filter(|p| !p.is_ai && p.active).collect()
    }

    fn get_real_active_players_mut(game: &mut Game) -> Vec<&mut Player> {
        game.players.iter_mut().filter(|p| !p.is_ai && p.active).collect()
    }
}

/// Make actor from `GameServer`
impl Actor for GameServer {
    /// We are going to use simple Context, we just need ability to communicate
    /// with other actors.
    type Context = Context<Self>;
}

impl actix::Supervised for GameServer {
    fn restarting(&mut self, ctx: &mut Context<GameServer>) {
        println!("restarting server after error");
    }
}

impl Handler<Connect> for GameServer {
    type Result = usize;

    fn handle(&mut self, connect: Connect, ctx: &mut Context<Self>) -> Self::Result {
        let current_game: &mut Game;
        let mut key: u32;
        let mut open_games: Vec<(_, &mut Game)>;

        if let Some(ref game_id) = connect.game_id {
            open_games = Vec::new();
            open_games.push((game_id, self.games.get_mut(&game_id).unwrap()));
        } else {
            open_games = self.games.iter_mut().filter(|(_, game)| !game.game_started).collect();
        }

        if open_games.len() > 0 {
            key = *open_games[0].0 as u32;
            current_game = open_games[0].1;
        } else {
            println!("didn't find open game");
            key = self.rng.gen();
            while self.games.contains_key(&(key as usize)) { key = self.rng.gen(); }

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

            let game_tick_handle = ctx.run_interval(Duration::from_millis(1000), move |act, ctx| {
                ctx.address().do_send(GameTickerMessage {
                    game_id: key as usize,
                });  
            }); 
            self.games.insert(key as usize, Game {
                board_size: BOARD_SIZE,
                game_started: false,
                players: Vec::new(),
                turn_index: 0,
                available_pos: available_pos.into_iter().collect(),
                game_countdown_handle: None,
                has_loop_countdown: false,
                game_tick_handle: game_tick_handle,
            });
            current_game = self.games.get_mut(&(key as usize)).unwrap();

            ctx.address().do_send(CountDownMessage {
                game_id: key as usize,
            });            
        }

        println!("{} joined", connect.username);
        if !current_game.available_pos.is_empty() {
            let pos = current_game.available_pos.remove(0);
            let player_id = current_game.players.len();
            current_game.players.push(Player::new(
                player_id,
                key as usize,
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

            for some_player in current_game.players.iter().map(|p| PlayerJoin::from(p)) {
                if let Ok(json_string) = serde_json::to_string(&some_player) {
                    if let Some(ref addr) = connect.addr {
                        let _ = addr.do_send(ToUserMessage(json_string));
                    }
                }
            }
            Self::broadcast_to_game(
                &PlayerJoin::from(current_player),
                Some(player_id),
                current_game.get_cloned_players_id_addr()
            );

            // if max players has reached start game
            if current_game.players.len() == MAX_PLAYERS && !current_game.game_started {
                ctx.address().do_send(CreateTurnMessage {
                    game_id: key as usize,
                    check_turn_id: None,
                });
            }

            if current_player.is_ai && current_player.active {
                ctx.address().do_send(MutationMessage {
                    mutation: Mutation {
                        user_id: player_id,
                        card_type: self.cardDist.sample(&mut self.rng) as u8,
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
                let _ = addr.do_send(ToUserMessage(format!("Game already full: {}", key)));
            }
            0
        }
    }
}

impl Handler<MutationMessage> for GameServer {
    type Result = usize;

    fn handle(&mut self, mutation: MutationMessage, _: &mut Context<Self>) -> Self::Result {
        if let Some(current_game) = self.games.get_mut(&mutation.game_id) {
            let addr_data = current_game.get_cloned_players_id_addr();
            let current_player = &mut current_game.players[mutation.player_id];

            if current_player.private_key == mutation.pk {
                if let Some(ref card_options) = current_player.card_options {
                    if card_options.contains(&mutation.mutation.card_type) || current_player.is_ai {
                        current_player.previous_choices.push(mutation.mutation);
                        Self::broadcast_to_game(
                            current_player.previous_choices.last().unwrap(),
                            None,
                            addr_data
                        );
                    } else {
                        if let Some(ref addr) = current_player.addr {
                            let _ = addr.do_send(ToUserMessage(String::from("Card not in option")));
                        }
                    }
                    current_player.card_options = None;
                }
            } else {
                if let Some(ref addr) = current_player.addr {
                    let _ = addr.do_send(ToUserMessage(String::from("Wrong pk")));
                }
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
        if let Some(current_game) = self.games.get_mut(&countdown.game_id) {
            current_game.game_countdown_handle = Some(ctx.run_interval(Duration::from_secs(1), move |_act, ctx| {
                let secs = Instant::now().duration_since(start);
                if secs <= Duration::from_secs(30) {
                    ctx.address().do_send(BroadcastStr {
                        json_string: format!("{{\"type\": \"TillStart\", \"secs\": \"{}\"}}", 30 - secs.as_secs()),
                        ori_player_id: None,
                        game_id: game_id
                    });
                } else {
                    ctx.address().do_send(CreateTurnMessage {
                        game_id: game_id,
                        check_turn_id: None,
                    });
                }
            }));
        }

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
        if let Some(current_game) = self.games.get_mut(&obj.game_id) {
            let p_data = current_game.get_cloned_players_id_addr();
            GameServer::broadcast_to_game_str(obj.json_string, obj.ori_player_id, p_data);
        }
    }
}

impl Handler<CreateTurnMessage> for GameServer {
    type Result = usize;

    fn handle(&mut self, gameinfo: CreateTurnMessage, ctx: &mut Context<Self>) -> Self::Result {
        if let Some(current_game) = self.games.get_mut(&gameinfo.game_id) {
            let mut rng = self.rng;
            current_game.has_loop_countdown = false;

            if let Some(check_turn_id) = gameinfo.check_turn_id {
                if check_turn_id != current_game.turn_index { return 0; }
            }

            if let Some(countdown_handle) = current_game.game_countdown_handle {
                ctx.cancel_future(countdown_handle);
                current_game.game_countdown_handle = None;
                current_game.game_started = true;
                GameServer::fill_slots_with_ai(current_game, &mut rng, ctx, gameinfo.game_id, &self.gc);
            } else {
                let active_players_with_no_move = current_game.players.iter().filter(|p| p.previous_choices.len() < current_game.turn_index && p.active);

                for player in active_players_with_no_move {
                    ctx.address().do_send(MutationMessage {
                        mutation: Mutation {
                            user_id: player.id,
                            card_type: player.card_options.as_ref().unwrap()[self.rng.gen_range(0, 3)],
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

            for ai_player in current_game.players.iter().filter(|p| p.is_ai && p.active) {
                ctx.address().do_send(MutationMessage {
                    mutation: Mutation {
                        user_id: ai_player.id,
                        card_type: self.cardDist.sample(&mut self.rng) as u8,
                        card_location: 0,
                    },
                    player_id: ai_player.id,
                    pk: ai_player.private_key.clone(),
                    game_id: ai_player.game_id,
                    turn_id: current_game.turn_index,
                });
            }
        }

        gameinfo.game_id
    }
}

impl Handler<CardChoiceMessage> for GameServer {
    type Result = usize;

    fn handle(&mut self, gameinfo: CardChoiceMessage, _: &mut Context<Self>) -> Self::Result {
        if let Some(current_game) = self.games.get_mut(&gameinfo.game_id) {
            for player in  current_game.players.iter_mut() {
                if player.active {
                    player.card_options = Some(vec![
                        self.cardDist.sample(&mut self.rng) as u8,
                        self.cardDist.sample(&mut self.rng) as u8,
                        self.cardDist.sample(&mut self.rng) as u8,
                    ]); 

                    let card_options = CardOptions {
                        card_options: player.card_options.as_ref().unwrap().clone(),
                        player_id: player.id,
                        turn_id: gameinfo.turn_id,
                    };
                    if let Ok(json_string) = serde_json::to_string(&card_options) {
                        if let Some(ref addr) = player.addr {
                            let _ = addr.do_send(ToUserMessage(json_string));
                        }
                    }
                }
            }
        }
        gameinfo.turn_id
    }
}

impl Handler<AnimationMessage> for GameServer {
    type Result = usize;

    fn handle(&mut self, gameinfo: AnimationMessage, ctx: &mut Context<Self>) -> Self::Result {
        if let Some(current_game) = self.games.get_mut(&gameinfo.game_id) {
            current_game.players[gameinfo.player_id].animation_done = true;

            // let players_with_animation_count = current_game
            //     .players
            //     .iter()
            //     .filter(|p| p.animation_done)
            //     .collect::<Vec<_>>().len();

            // let game_id = gameinfo.game_id;
            // let active_player_count = Self::get_real_active_players(current_game).len();

            // if players_with_animation_count == MAX_PLAYERS - active_player_count {
            //     ctx.address().do_send(CreateTurnMessage {
            //         game_id: game_id,
            //         check_turn_id: None,
            //     });
            // } else if players_with_animation_count as f64 >= (active_player_count as f64 * 0.5f64) && !current_game.has_loop_countdown {
            //     current_game.has_loop_countdown = true;
            //     let current_turn_index = current_game.turn_index;
            //     ctx.run_later(Duration::from_secs(10), move |_act, ctx| {
            //         ctx.address().do_send(CreateTurnMessage {
            //             game_id: game_id,
            //             check_turn_id: Some(current_turn_index),
            //         });
            //     });
            // }
        } else {
            println!("Animation recived after game end id: {}", gameinfo.game_id);
        }

        gameinfo.player_id
    }
}

impl Handler<PollPlayerDeathMessage> for GameServer {
    type Result = usize;

    fn handle(&mut self, gameinfo: PollPlayerDeathMessage, ctx: &mut Context<Self>) -> Self::Result {
        if let Some(current_game) = self.games.get_mut(&gameinfo.game_id) {
            let addr_data = current_game.get_cloned_players_id_addr();

            if !current_game.players[gameinfo.self_id].has_death_voted && gameinfo.pk == current_game.players[gameinfo.self_id].private_key && current_game.players[gameinfo.other_id].active {
                current_game.players[gameinfo.self_id].has_death_voted = true;
                current_game.players[gameinfo.other_id].been_death_voted_for += 1;

                let active_player_count = Self::get_real_active_players(current_game).len();

                if current_game.players[gameinfo.other_id].been_death_voted_for as f64 >= active_player_count as f64 * 0.5f64 {
                    Self::broadcast_to_game(&PlayerDied{user_id: gameinfo.other_id}, None, addr_data);
                    current_game.players[gameinfo.other_id].active = false;
                }
            }
        } else {
            println!("Death info message recived after game end id: {}", gameinfo.self_id);
        }

        gameinfo.self_id
    }
}

impl Handler<GameTickerMessage> for GameServer {
    type Result = usize;

    fn handle(&mut self, gameinfo: GameTickerMessage, ctx: &mut Context<Self>) -> Self::Result {
        if let Some(current_game) = self.games.get_mut(&gameinfo.game_id) {
            if current_game.game_started {
                let addr_data = current_game.get_cloned_players_id_addr();

                let mut all_players: Vec<_> = current_game.players.iter_mut().collect();
                { // Mark discconnected players as inactive
                    let marked_active_players = all_players.iter_mut().filter(|p| p.active);
                    for player in marked_active_players {
                        if let Some(ref addr) = player.addr {
                            if !addr.connected() {
                                player.active = false;
                                Self::broadcast_to_game(
                                    &PlayerDied{user_id: player.id},
                                    None,
                                    addr_data.clone()
                                );
                            }
                        }
                    }
                }

                let connected_players_havent_sent_animation = all_players
                    .iter().filter(|p| {
                        if let Some(ref addr) = p.addr {
                            addr.connected() && !p.animation_done
                        } else {
                            false
                        }
                    }).collect::<Vec<_>>().len();

                // If all players have sent an animation
                if connected_players_havent_sent_animation == 0 {
                    let connected_real_players = all_players
                    .iter().filter(|p| {
                        if let Some(ref addr) = p.addr {
                            addr.connected() && !p.is_ai
                        } else {
                            false
                        }
                    }).collect::<Vec<_>>().len();

                    // Delete the game if no players are connected
                    if connected_real_players == 0 {
                        ctx.address().do_send(DeleteGame {
                            game_id: gameinfo.game_id,
                            game_tick_handle: current_game.game_tick_handle,
                        });
                    }

                    let alive_players = all_players
                        .iter()
                        .filter(|p| p.active)
                        .collect::<Vec<_>>().len();

                    // If there is only one alive player end game
                    if alive_players == 1 {
                        ctx.address().do_send(DeleteGame {
                            game_id: gameinfo.game_id,
                            game_tick_handle: current_game.game_tick_handle,
                        });
                    } else { // Otherwise create a new turn
                        for player in all_players {
                            player.animation_done = false;
                        }
                        ctx.address().do_send(CreateTurnMessage {
                            game_id: gameinfo.game_id,
                            check_turn_id: None,
                        });
                    }
                }
            }
        } 
        
        // let game_started = current_game.game_started;
        // let addr_data = current_game.get_cloned_players_id_addr();
        // let alive_player_count = current_game.players.iter().filter(|p| p.active).collect::<Vec<_>>().len();
        // if alive_player_count == 1 && game_started {
        //     println!("{} has concluded", gameinfo.game_id);
        //     ctx.cancel_future(current_game.game_tick_handle);
        //     self.games.remove(&gameinfo.game_id);
        // } else {
        //     let mut connected_players = current_game.players.iter_mut().filter(|p| {
        //         if let Some(ref addr) = p.addr {
        //             addr.connected()
        //         } else {
        //             false
        //         }
        //     }).collect::<Vec<_>>();

        //     let animation_waiting = connected_players.iter().filter(|p| !p.animation_done).collect::<Vec<_>>().len();

        //     println!("Waiting for animation from {} players", animation_waiting);
        //     if animation_waiting == 0 {
                
        //         for player in connected_players {
        //             player.animation_done = false;
        //         }
        //         ctx.address().do_send(CreateTurnMessage {
        //             game_id: gameinfo.game_id,
        //             check_turn_id: None,
        //         });
        //     } else {
        //         let real_alive_players = connected_players.iter_mut().filter(|p| p.active);
        //         for player in real_alive_players {
        //             if let Some(ref addr) = player.addr {
        //                 if !addr.connected() {
        //                     println!("{} left :(", player.username);
        //                     player.active = false;
        //                     Self::broadcast_to_game(
        //                         &PlayerDied{user_id: player.id},
        //                         None,
        //                         addr_data.clone()
        //                     );
        //                 }
        //             }
        //         }
        //     }
        // }

        gameinfo.game_id
    }
}

impl Handler<DeleteGame> for GameServer {
    type Result = usize;

    fn handle(&mut self, gameinfo: DeleteGame, ctx: &mut Context<Self>) -> Self::Result {
        println!("{} has concluded", gameinfo.game_id);
        ctx.cancel_future(gameinfo.game_tick_handle);
        self.games.remove(&gameinfo.game_id);

        gameinfo.game_id
    }
}

impl Handler<MessagePrune> for GameServer {
    type Result = usize;

    fn handle(&mut self, msg: MessagePrune, _: &mut Context<Self>) -> Self::Result {
        let mut to_remove = Vec::new();

        for (i, game) in self.games.iter() {
            let active_players = Self::get_real_active_players(game).len();
            if active_players < 1 {
                to_remove.push(*i);
            }
        }

        for i in to_remove.iter() {
            self.games.remove(i);
        }

        let _ = msg.addr.do_send(ToUserMessage(format!("Removed {} games", to_remove.len())));
        to_remove.len()
    }
}

impl Handler<MessageFullClean> for GameServer {
    type Result = usize;

    fn handle(&mut self, msg: MessageFullClean, _: &mut Context<Self>) -> Self::Result {
        let keys: Vec<_> = self.games.keys().into_iter().map(|i| *i).collect();

        for i in keys.iter() {
            self.games.remove(i);
        }

        let _ = msg.addr.do_send(ToUserMessage(format!("Removed {} games", keys.len())));
        keys.len()
    }
}

impl Handler<Disconnect> for GameServer {
    type Result = i32;

    fn handle(&mut self, msg: Disconnect, _: &mut Context<Self>) -> Self::Result {
        // println!("Someone left");
        0
    }
}
