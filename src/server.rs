use std::time::{Duration, Instant};
use std::collections::{HashMap, HashSet};
use actix::prelude::*;
use serde::Serialize;
use rand::{thread_rng, Rng, prelude::ThreadRng, distributions::Alphanumeric};
use crate::models::{Game, Turn, Player, Mutation, CardOptions};

use diesel::prelude::*;

use crate::DbPool;

const NUMBER_OF_CARDS: i32 = 50i32;
const MAX_PLAYERS: i32 = 30i32;
const NUMBER_OF_CHAR_TYPES: i32 = 5i32;

#[derive(Message)]
#[rtype(result = "()")]
pub struct JsonStringMessage(pub String);

/// New chat session is created
#[derive(Message)]
#[rtype(i32)]
pub struct Connect {
    pub addr: Option<Recipient<JsonStringMessage>>,
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
}

#[derive(Message)]
#[rtype(i32)]
pub struct CreateTurnMessage {
    pub game_id: i32,
}

#[derive(Message)]
#[rtype(i32)]
pub struct CardChoiceMessage {
    pub game_id: i32,
    pub turn_id: i32,
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

#[derive(Message)]
#[rtype(result = "()")]
pub struct BroadcastObj<O: Serialize> {
    pub serializabe_obj: O,
    pub ori_player_id: Option<i32>,
    pub game_id: i32,
}

#[derive(Message)]
#[rtype(result = "()")]
pub struct BroadcastStr {
    pub json_string: String,
    pub ori_player_id: Option<i32>,
    pub game_id: i32,
}

#[derive(Message)]
#[rtype(i32)]
pub struct AnimationMessage {
    pub player_id: i32,
    pub pk: i32,
    pub turn_id: i32,
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
    games: Vec<Game>,
    player_sessions: HashMap<i32, Option<Recipient<JsonStringMessage>>>,
    animation_percentages: HashMap<i32, i32>,
    pool: DbPool,
    rng: ThreadRng,
    countdown_handle: Option<SpawnHandle>,
}

impl GameServer {
    pub fn new(pool: DbPool) -> GameServer {
        GameServer {
            games: Vec::new(),
            player_sessions: HashMap::new(),
            animation_percentages: HashMap::new(),
            pool: pool,
            rng: thread_rng(),
            countdown_handle: None,
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
                            if let Some(session) = session {
                                let _ = session.do_send(JsonStringMessage(mutation));
                            }
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
            self.broadcast_to_game_str(json_string, ori_player_id, game_id_val);
        }
    }

    fn broadcast_to_game_str(
        &self, json_string: String,
        ori_player_id: Option<i32>,
        game_id_val: i32
    ) {
        let conn = self.pool.get().expect("couldn't get db connection from pool");
        use crate::schema::player::dsl::{player, game_id};

        let players = player.filter(game_id.eq(game_id_val)).load::<Player>(&conn);
        if let Ok(players) = players {
            for some_player in players.iter() {
                let json_string = json_string.clone();
                match ori_player_id {
                    Some(ori_player_id) if some_player.id != ori_player_id => {
                        if let Some(session) = self.player_sessions.get(&some_player.id) {
                            if let Some(session) = session {
                                let _ = session.do_send(JsonStringMessage(json_string));
                            }
                        }
                    },
                    None => {
                        if let Some(session) = self.player_sessions.get(&some_player.id) {
                            if let Some(session) = session {
                                let _ = session.do_send(JsonStringMessage(json_string));
                            }
                        }
                    },
                    _ => ()
                }
            }
        }
    }

    fn fill_slots_with_ai(&mut self, game_id_val: i32, ctx: &mut Context<Self>) {
        let conn = self.pool.get().expect("couldn't get db connection from pool");

        use crate::schema::player::dsl::*;
        let current_players = player.filter(game_id.eq(game_id_val)).get_results::<Player>(&conn);
        if let Ok(current_players) = current_players {
            let mut ai_players = vec![];
            let mut postistion_set = HashSet::new();

            for i in 0..(MAX_PLAYERS - current_players.len() as i32) {
                let mut pos: (i32, i32) = (self.rng.gen_range(0, 100), self.rng.gen_range(0, 100));

                while {
                    !postistion_set.contains(&pos) && match player
                        .filter(pos_x.eq(pos.0))
                        .filter(pos_y.eq(pos.1))
                        .load::<Player>(&conn) {
                        Ok(players) => !players.is_empty(),
                        Err(_) => false,
                    }
                } {
                    pos.0 = self.rng.gen_range(0, 100);
                    pos.1 = self.rng.gen_range(0, 100);
                }

                postistion_set.insert(pos);
                ai_players.push((
                    private_key.eq(self.rng
                        .sample_iter(&Alphanumeric)
                        .take(10)
                        .collect::<String>()),
                    username.eq(format!("Bot {}", i)),
                    character_type.eq(self.rng.gen_range(0, NUMBER_OF_CHAR_TYPES)),
                    pos_x.eq(pos.0),
                    pos_y.eq(pos.1),
                    pos_orientation.eq(self.rng.gen_range(0, 4)),
                    is_ai.eq(false),
                    game_id.eq(game_id_val)
                ));
            }

            let ai_players: Result<Vec<Player>, _> = diesel::insert_into(player).values(&ai_players).get_results(&conn);

            if let Ok(mut ai_players) = ai_players {
                while !ai_players.is_empty() {
                    ctx.address().do_send(Connect {
                        addr: None,
                        player: ai_players.remove(0)
                    });
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

    fn handle(&mut self, msg: Connect, ctx: &mut Context<Self>) -> Self::Result {
        println!("{} joined", msg.player.username);
        let conn = self.pool.get().expect("couldn't get db connection from pool");

        use crate::schema::player::dsl::{player, game_id};
        let players_in_game = player
            .filter(game_id.eq(msg.player.game_id))
            .load::<Player>(&conn);
        let mut start_game = false;
        if let Ok(players) = players_in_game {
            start_game = players.len() as i32 == MAX_PLAYERS;
            for some_player in  players.iter().map(|x| PlayerJoin::from(x)) {
                if let Ok(json_string) = serde_json::to_string(&some_player) {
                    if let Some(ref addr) = msg.addr {
                        let _ = addr.do_send(JsonStringMessage(json_string));
                    }
                }
            }
        }
        self.player_sessions.insert(msg.player.id, msg.addr);
        self.broadcast_to_game(&PlayerJoin::from(&msg.player), Some(msg.player.id), msg.player.game_id);

        // if max players is reached start game
        if start_game {
            if let Some(_) = self.countdown_handle {
                ctx.address().do_send(CreateTurnMessage {
                    game_id: msg.player.game_id,
                });
            }
        }

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

    fn handle(&mut self, countdown: CountDownMessage, ctx: &mut Context<Self>) -> Self::Result {
        let start = Instant::now();
        let game_id = countdown.game_id;
        self.countdown_handle = Some(ctx.run_interval(Duration::from_secs(1), move |_act, ctx| {
            let secs = Instant::now().duration_since(start);
            if secs <= Duration::from_secs(10) {
                ctx.address().do_send(BroadcastStr {
                    json_string: format!("{{\"type\": \"TillStart\", \"secs\": \"{}\"}}", 10 - secs.as_secs()),
                    ori_player_id: None,
                    game_id: game_id
                });
            } else {
                ctx.address().do_send(CreateTurnMessage {
                    game_id: game_id,
                });
            }
        }));

        countdown.game_id
    }
}

impl<O: Serialize> Handler<BroadcastObj<O>> for GameServer {
    type Result = ();

    fn handle(&mut self, obj: BroadcastObj<O>, _: &mut Context<Self>) {
        self.broadcast_to_game(&obj.serializabe_obj, obj.ori_player_id, obj.game_id);
    }
}

impl Handler<BroadcastStr> for GameServer {
    type Result = ();

    fn handle(&mut self, obj: BroadcastStr, _: &mut Context<Self>) {
        self.broadcast_to_game_str(obj.json_string, obj.ori_player_id, obj.game_id);
    }
}

impl Handler<CreateTurnMessage> for GameServer {
    type Result = i32;

    fn handle(&mut self, gameinfo: CreateTurnMessage, ctx: &mut Context<Self>) -> Self::Result {
        if let Some(countdown_handle) = self.countdown_handle {
            ctx.cancel_future(countdown_handle);
            self.countdown_handle = None;
        }
        
        let conn = self.pool.get().expect("couldn't get db connection from pool");
        use crate::schema::game::dsl::{game, game_started};

        // Create ai players
        self.fill_slots_with_ai(gameinfo.game_id, ctx);

        use crate::schema::turn::dsl::{turn, game_id as turn_game_id};
        let new_turn = diesel::insert_into(turn)
            .values(turn_game_id.eq(gameinfo.game_id))
            .get_result::<Turn>(&conn);
        
        match new_turn {
            Ok(new_turn) => {
                let _ = diesel::update(game.find(gameinfo.game_id)).set(game_started.eq(true)).execute(&conn);
                self.broadcast_to_game(&new_turn, None, gameinfo.game_id);
                ctx.address().do_send(CardChoiceMessage {
                    game_id: gameinfo.game_id,
                    turn_id: new_turn.id,
                });
            },
            Err(_) => (),
        };

        gameinfo.game_id
    }
}

impl Handler<CardChoiceMessage> for GameServer {
    type Result = i32;

    fn handle(&mut self, gameinfo: CardChoiceMessage, _: &mut Context<Self>) -> Self::Result {
        let conn = self.pool.get().expect("couldn't get db connection from pool");

        use crate::schema::player::dsl::{player, game_id};
        let players_in_game = player
            .filter(game_id.eq(gameinfo.game_id))
            .load::<Player>(&conn);
        if let Ok(players) = players_in_game {
            use crate::schema::card_options_table::dsl::*;
            for some_player in  players.iter() {
                let card_option_cal: Vec<i32> = vec![
                    self.rng.gen_range(0, NUMBER_OF_CARDS),
                    self.rng.gen_range(0, NUMBER_OF_CARDS),
                    self.rng.gen_range(0, NUMBER_OF_CARDS),
                ]; 
                let cards: Result<CardOptions, _> = diesel::insert_into(card_options_table).values((
                    card_options.eq(card_option_cal),
                    player_id.eq(some_player.id),
                    turn_id.eq(gameinfo.turn_id),
                )).get_result(&conn);
                match cards {
                    Ok(cards) => {
                        match self.player_sessions.get(&some_player.id) {
                            Some(player_connection) => {
                                if let Ok(json_string) = serde_json::to_string(&cards) {
                                    if let Some(player_connection) = player_connection {
                                        let _ = player_connection.do_send(JsonStringMessage(json_string));
                                    }
                                }
                            },
                            None => (),
                        }
                    },
                    _ => ()
                }
            }
        }
        gameinfo.turn_id
    }
}

impl Handler<Disconnect> for GameServer {
    type Result = i32;

    fn handle(&mut self, msg: Disconnect, _: &mut Context<Self>) -> Self::Result {
        println!("Someone left");
        0
    }
}
