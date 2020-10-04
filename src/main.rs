#[macro_use]
extern crate diesel;

use std::time::{Duration, Instant};
use rand::{thread_rng, Rng};
use rand::distributions::Alphanumeric;

use actix::prelude::*;
use actix_web::{web, App, Error, HttpRequest, HttpResponse, HttpServer, Responder};
use actix_web_actors::ws;

use diesel::{prelude::*, r2d2::{self, ConnectionManager}};

use serde::{Deserialize, Serialize};

mod models;
mod schema;
mod server;

type DbPool = r2d2::Pool<ConnectionManager<PgConnection>>;

/// How often heartbeat pings are sent
const HEARTBEAT_INTERVAL: Duration = Duration::from_secs(5);
/// How long before lack of client response causes a timeout
const CLIENT_TIMEOUT: Duration = Duration::from_secs(10);

const NUMBER_OF_CARDS: i32 = 50i32;


struct GameWebSocket {
    id: usize,
    hb: Instant,
    data: (DbPool, Addr<server::GameServer>),
}

impl Actor for GameWebSocket {
    type Context = ws::WebsocketContext<Self>;

    fn started(&mut self, ctx: &mut Self::Context) {
        // self.hb(ctx);

        let addr = ctx.address();
        self.data.1
            .send(server::Connect {
                addr: addr.recipient(),
                pool: self.data.0.clone(),
            })
            .into_actor(self)
            .then(|res, act, ctx| {
                match res {
                    Ok(res) => act.id = res,
                    // something is wrong with chat server
                    _ => ctx.stop(),
                }
                fut::ready(())
            })
            .wait(ctx);
    }

    fn stopping(&mut self, _: &mut Self::Context) -> Running {
        // notify chat server
        self.data.1.do_send(server::Disconnect { id: self.id });
        Running::Stop
    }
}

impl Handler<server::JsonStringMessage> for GameWebSocket {
    type Result = ();

    fn handle(&mut self, msg: server::JsonStringMessage, ctx: &mut Self::Context) {
        ctx.text(msg.0);
    }
}

#[derive(Deserialize)]
#[serde(tag = "type")]
pub enum MessageType {
    InitiateGame {
        username: String,
        charater_type: i32,
        color: Option<i32>,
    },
    ChooseCard {
        card_number: i32,
        location: i32,
        user_id: i32,
        pk: String,
    },
}

/// Handler for ws::Message message
impl StreamHandler<Result<ws::Message, ws::ProtocolError>> for GameWebSocket {
    fn handle(
        &mut self,
        msg: Result<ws::Message, ws::ProtocolError>,
        ctx: &mut Self::Context,
    ) {
        match msg {
            Ok(ws::Message::Ping(msg)) => ctx.pong(&msg),
            Ok(ws::Message::Text(text)) => {
                match serde_json::from_str(text.as_str()).unwrap() {
                    MessageType::InitiateGame{username, charater_type, color} => {
                        let username_sent = username;
                        use crate::schema::game::dsl::*;
                        let conn = self.data.0.get().expect("couldn't get db connection from pool");
                        
                        let current_games = game
                            .filter(game_started.eq(false))
                            .load::<models::Game>(&conn);
                        
                        match current_games {
                            Ok(mut current_games) => {
                                let mut current_game = None;
                                if current_games.len() > 0 {
                                    current_game = Some(current_games.remove(0));
                                } else {
                                    let new_game = diesel::insert_into(game)
                                        .default_values()
                                        .get_result(&conn);
                                    match new_game {
                                        Ok(new_game) => {
                                            current_game = Some(new_game);
                                        },
                                        Err(e) => ctx.text(format!("{}", e)),
                                    }
                                }

                                if let Some(current_game) = current_game {
                                    use crate::schema::player::dsl::*;

                                    let new_player: Result<models::Player, _> = diesel::insert_into(player)
                                        .values((
                                            private_key.eq(thread_rng()
                                                .sample_iter(&Alphanumeric)
                                                .take(10)
                                                .collect::<String>()),
                                            username.eq(&username_sent),
                                            character_type.eq(&charater_type),
                                            is_ai.eq(false),
                                            game_id.eq(current_game.id)
                                        ))
                                        .get_result(&conn);
                                    match new_player {
                                        Ok(new_player) => match serde_json::to_string(&new_player) {
                                            Ok(s) => ctx.text(s),
                                            Err(e) => ctx.text(format!("{}", e)),
                                        },
                                        Err(e) => ctx.text(format!("{}", e)),
                                    }
                                } else {
                                    ctx.text("kinda wierd")
                                }
                            },
                            Err(e) => ctx.text(format!("{}", e)),
                        }
                    },
                    MessageType::ChooseCard {card_number, location, user_id, pk} => {

                    }
                    _ => ()
                }
            },
            Ok(ws::Message::Binary(bin)) => ctx.binary(bin),
            _ => (),
        }
    }
}

async fn index(
    req: HttpRequest, stream: web::Payload, data: web::Data<(DbPool, Addr<server::GameServer>)>,
) -> Result<HttpResponse, Error> {
    let resp = ws::start(GameWebSocket {
        id: 0,
        hb: Instant::now(),
        data: data.get_ref().clone(),
    }, &req, stream);
    println!("{:?}", resp);
    resp
}

// impl GameWebSocket {
//     /// helper method that sends ping to client every second.
//     ///
//     /// also this method checks heartbeats from client
//     fn hb(&self, ctx: &mut ws::WebsocketContext<Self>) {
//         ctx.run_interval(HEARTBEAT_INTERVAL, |act, ctx| {
//             // check client heartbeats
//             if Instant::now().duration_since(act.hb) > CLIENT_TIMEOUT {
//                 // heartbeat timed out
//                 println!("Websocket Client heartbeat failed, disconnecting!");

//                 // notify chat server
//                 act.data.1.do_send(server::Disconnect { id: act.id });

//                 // stop actor
//                 ctx.stop();

//                 // don't try to send a ping
//                 return;
//             }

//             ctx.ping(b"");
//         });
//     }
// }

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    // set up database connection pool
    let connspec = std::env::var("DATABASE_URL").expect("DATABASE_URL");
    let manager = ConnectionManager::<PgConnection>::new(connspec);
    let pool = r2d2::Pool::builder()
        .build(manager)
        .expect("Failed to create pool.");

    let server = server::GameServer::new(pool.clone()).start();

    HttpServer::new(move || {
        App::new()
        .data((pool.clone(), server.clone()))
        .route("/game/", web::get().to(index))
    })
        .bind("0.0.0.0:8080")?
        .run()
        .await
}
