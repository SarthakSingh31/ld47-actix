use std::collections::{HashMap, HashSet};
use actix::prelude::*;
use crate::models::{Game, Turn, Player, Mutation, CardOptions};

use diesel::{prelude::*, r2d2::{self, ConnectionManager}};

use crate::DbPool;

#[derive(Message)]
#[rtype(result = "()")]
pub struct JsonStringMessage(pub String);

/// New chat session is created
#[derive(Message)]
#[rtype(usize)]
pub struct Connect {
    pub addr: Recipient<JsonStringMessage>,
    pub pool: DbPool,
}

/// Session is disconnected
#[derive(Message)]
#[rtype(usize)]
pub struct Disconnect {
    pub id: usize,
}

pub struct ListGames;

impl actix::Message for ListGames {
    type Result = Vec<Game>;
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

    fn relay_mutation(&self, game: &Game, mutation: &Mutation, skip_id: i32) {
        let conn = self.pool.get().expect("couldn't get db connection from pool");

        use crate::schema::player::dsl::{player, game_id};

        let players = player.filter(game_id.eq(game.id)).load::<Player>(&conn);
        if let Ok(players) = players {
            for some_player in players.iter() {
                if some_player.id != skip_id {
                    if let Some(session) = self.player_sessions.get(&some_player.id) {
                        if let Ok(mutation) = serde_json::to_string(&mutation) {
                            let _ = session.do_send(JsonStringMessage(mutation));
                        }
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
    type Result = usize;

    fn handle(&mut self, msg: Connect, _: &mut Context<Self>) -> Self::Result {
        println!("Someone joined");
        0
    }
}

impl Handler<Disconnect> for GameServer {
    type Result = usize;

    fn handle(&mut self, msg: Disconnect, _: &mut Context<Self>) -> Self::Result {
        println!("Someone left");
        0
    }
}