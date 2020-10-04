use std::collections::{HashMap, HashSet};
use actix::prelude::*;
use serde::Serialize;
use crate::models::{Game, Turn, Player, Mutation, CardOptions};

use diesel::prelude::*;

use crate::DbPool;

#[derive(Message)]
#[rtype(result = "()")]
pub struct JsonStringMessage(pub String);

/// New chat session is created
#[derive(Message)]
#[rtype(i32)]
pub struct Connect {
    pub addr: Recipient<JsonStringMessage>,
    pub player: Player,
}

#[derive(Message)]
#[rtype(i32)]
pub struct MutationMessage {
    pub mutation: Mutation,
    pub player_id: i32,
    pub game_id: i32,
}

#[derive(Message)]
#[rtype(i32)]
pub struct CountDownMessage {
    pub game_id: i32,
    pub secs: i32,
}

#[derive(Message)]
#[rtype(i32)]
pub struct CreateTurnMessage {
    pub game_id: i32,
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
    pub user_id: i32,
    pub username: String,
    pub x: i32,
    pub y: i32,
    pub char_type: i32,
    pub start_orientation: i32,
}

// TODO: put real player pos here
impl From<&Player> for PlayerJoin {
    fn from(player: &Player) -> Self {
        PlayerJoin {
            user_id: player.id,
            username: player.username.clone(),
            x: player.pos_x,
            y: player.pos_y,
            char_type: player.character_type,
            start_orientation: player.pos_orientation,
        }
    }
}

pub struct GameServer {
    player_sessions: HashMap<i32, Recipient<JsonStringMessage>>,
    pool: DbPool,
}

impl GameServer {
    pub fn new(pool: DbPool) -> GameServer {
        GameServer {
            player_sessions: HashMap::new(),
            pool: pool,
        }
    }

    fn relay_mutation(&self, mutation: &Mutation, ori_player_id: i32, game_id_val: i32) {
        let conn = self.pool.get().expect("couldn't get db connection from pool");

        use crate::schema::player::dsl::{player, game_id};

        let players = player.filter(game_id.eq(game_id_val)).load::<Player>(&conn);
        if let Ok(players) = players {
            for some_player in players.iter() {
                if some_player.id != ori_player_id {
                    if let Some(session) = self.player_sessions.get(&some_player.id) {
                        if let Ok(mutation) = serde_json::to_string(&mutation) {
                            let _ = session.do_send(JsonStringMessage(mutation));
                        }
                    }
                }
            }
        }
    }

    fn broadcast_to_game<O: Serialize>(
        &self, serializabe_obj: &O,
        ori_player_id: Option<i32>,
        game_id_val: i32
    ) {
        if let Ok(json_string) = serde_json::to_string(serializabe_obj) {
            let conn = self.pool.get().expect("couldn't get db connection from pool");
            use crate::schema::player::dsl::{player, game_id};

            let players = player.filter(game_id.eq(game_id_val)).load::<Player>(&conn);
            if let Ok(players) = players {
                for some_player in players.iter() {
                    let json_string = json_string.clone();
                    match ori_player_id {
                        Some(ori_player_id) if some_player.id != ori_player_id => {
                            if let Some(session) = self.player_sessions.get(&some_player.id) {
                                let _ = session.do_send(JsonStringMessage(json_string));
                            }
                        },
                        None => {
                            if let Some(session) = self.player_sessions.get(&some_player.id) {
                                let _ = session.do_send(JsonStringMessage(json_string));
                            }
                        },
                        _ => ()
                    }
                }
            }
        }
    }
}

/// Make actor from `GameServer`
impl Actor for GameServer {
    /// We are going to use simple Context, we just need ability to communicate
    /// with other actors.
    type Context = Context<Self>;
}

impl Handler<Connect> for GameServer {
    type Result = i32;

    fn handle(&mut self, msg: Connect, _: &mut Context<Self>) -> Self::Result {
        println!("{} joined", msg.player.username);
        let conn = self.pool.get().expect("couldn't get db connection from pool");

        use crate::schema::player::dsl::{player, game_id};
        let players_in_game = player
            .filter(game_id.eq(msg.player.game_id))
            .load::<Player>(&conn);
        if let Ok(players) = players_in_game {
            for some_player in  players.iter().map(|x| PlayerJoin::from(x)) {
                if let Ok(json_string) = serde_json::to_string(&some_player) {
                    let _ = msg.addr.do_send(JsonStringMessage(json_string));
                }
            }
        }
        self.player_sessions.insert(msg.player.id, msg.addr);
        self.broadcast_to_game(&PlayerJoin::from(&msg.player), Some(msg.player.id), msg.player.game_id);

        msg.player.id
    }
}

impl Handler<MutationMessage> for GameServer {
    type Result = i32;

    fn handle(&mut self, mutation: MutationMessage, _: &mut Context<Self>) -> Self::Result {
        self.relay_mutation(&mutation.mutation, mutation.player_id, mutation.game_id);
        mutation.player_id
    }
}

impl Handler<CountDownMessage> for GameServer {
    type Result = i32;

    fn handle(&mut self, countdown: CountDownMessage, _: &mut Context<Self>) -> Self::Result {
        let conn = self.pool.get().expect("couldn't get db connection from pool");

        use crate::schema::player::dsl::{player, game_id};

        let players = player.filter(game_id.eq(countdown.game_id)).load::<Player>(&conn);
        if let Ok(players) = players {
            for some_player in players.iter() {
                if let Some(session) = self.player_sessions.get(&some_player.id) {
                    let _ = session.do_send(JsonStringMessage(
                        format!("{{\"type\": \"TillStart\", \"secs\": \"{}\"}}", countdown.secs)
                    ));
                }
            }
        }

        countdown.secs
    }
}

impl Handler<CreateTurnMessage> for GameServer {
    type Result = i32;

    fn handle(&mut self, gameinfo: CreateTurnMessage, _: &mut Context<Self>) -> Self::Result {
        let conn = self.pool.get().expect("couldn't get db connection from pool");
        use crate::schema::game::dsl::{game, game_started};

        use crate::schema::turn::dsl::{turn, game_id as turn_game_id};
        let new_turn = diesel::insert_into(turn)
            .values(turn_game_id.eq(gameinfo.game_id))
            .get_result::<Turn>(&conn);
        
        match new_turn {
            Ok(new_turn) => {
                let _ = diesel::update(game.find(gameinfo.game_id)).set(game_started.eq(true)).execute(&conn);
                use crate::schema::player::dsl::{player, game_id};

                let players = player.filter(game_id.eq(gameinfo.game_id)).load::<Player>(&conn);
                if let Ok(players) = players {
                    for some_player in players.iter() {
                        if let Some(session) = self.player_sessions.get(&some_player.id) {
                            let _ = match serde_json::to_string(&new_turn) {
                                Ok(new_turn_str) => session.do_send(JsonStringMessage(new_turn_str)),
                                Err(e) => session.do_send(JsonStringMessage(format!("{}", e))),
                            };
                        }
                    }
                }
            },
            Err(_) => (),
        };

        gameinfo.game_id
    }
}

impl Handler<Disconnect> for GameServer {
    type Result = i32;

    fn handle(&mut self, msg: Disconnect, _: &mut Context<Self>) -> Self::Result {
        println!("Someone left");
        0
    }
}
