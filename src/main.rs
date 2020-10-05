use std::env;

use actix::prelude::*;
use actix_web::{web, App, Error, HttpRequest, HttpResponse, HttpServer};
use actix_web_actors::ws;

use serde::{Deserialize};
use openssl::ssl::{SslAcceptor, SslFiletype, SslMethod};

mod models;
mod config;
mod server;

struct GameWebSocket {
    id: usize,
    data: Addr<server::GameServer>,
}

impl Actor for GameWebSocket {
    type Context = ws::WebsocketContext<Self>;

    fn stopping(&mut self, _: &mut Self::Context) -> Running {
        // notify chat server
        self.data.do_send(server::Disconnect { id: self.id });
        Running::Stop
    }
}

impl Handler<server::ToUserMessage> for GameWebSocket {
    type Result = ();

    fn handle(&mut self, msg: server::ToUserMessage, ctx: &mut Self::Context) {
        ctx.text(msg.0);
    }
}

#[derive(Deserialize)]
#[serde(tag = "type")]
pub enum MessageType {
    InitiateGame {
        username: String,
        character_type: u8,
        color: Option<u8>,
    },
    ChooseCard {
        card_number: u8,
        location: u8,
        player_id: usize,
        pk: String,
        turn_id: usize,
        game_id: usize,
    },
    AnimationsDone {
        player_id: usize,
        pk: String,
        turn_id: usize,
        game_id: usize,
    },
    PollPlayerDied {
        other_id: usize,
        self_id: usize,
        pk: String,
        turn_id: usize,
        game_id: usize,
    },
    Prune {
        password: String,
    },
    FullClean {
        password: String,
    }
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
                    MessageType::InitiateGame{username, character_type, color} => {
                        self.data.send(server::Connect {
                            username: username,
                            character_type: character_type,
                            addr: Some(ctx.address().recipient()),
                            game_id: None, // Only for bot use
                        })
                        .into_actor(self)
                        .then(|res, act, ctx| {
                            match res {
                                Ok(res) => act.id = res as usize,
                                // something is wrong with server
                                _ => ctx.stop(),
                            }
                            fut::ready(())
                        })
                        .wait(ctx);
                    },
                    MessageType::ChooseCard {card_number, location, player_id, pk, turn_id, game_id} => {
                        let mutation = models::Mutation {
                            card_type: card_number,
                            card_location: location,
                        };
                        self.data.send(server::MutationMessage {
                            mutation: mutation,
                            player_id: player_id,
                            game_id: game_id,
                            pk: pk,
                            turn_id: turn_id,
                        })
                        .into_actor(self)
                        .then(|res, act, ctx| {
                            match res {
                                Ok(res) => act.id = res as usize,
                                // something is wrong with server
                                _ => ctx.stop(),
                            }
                            fut::ready(())
                        })
                        .wait(ctx);
                    },
                    MessageType::AnimationsDone {player_id, pk, game_id, turn_id} => {
                        self.data.send(server::AnimationMessage {
                            player_id: player_id,
                            pk: pk,
                            game_id: game_id,
                            turn_id: turn_id,
                        })
                        .into_actor(self)
                        .then(|res, act, ctx| {
                            match res {
                                Ok(res) => act.id = res as usize,
                                // something is wrong with server
                                _ => ctx.stop(),
                            }
                            fut::ready(())
                        })
                        .wait(ctx);
                    },
                    MessageType::PollPlayerDied {other_id, self_id, pk, game_id, turn_id} => {
                        self.data.send(server::PollPlayerDeathMessage {
                            other_id: other_id,
                            self_id: self_id,
                            pk: pk,
                            game_id: game_id,
                            turn_id: turn_id,
                        })
                        .into_actor(self)
                        .then(|res, act, ctx| {
                            match res {
                                Ok(res) => act.id = res as usize,
                                // something is wrong with server
                                _ => ctx.stop(),
                            }
                            fut::ready(())
                        })
                        .wait(ctx);
                    },
                    MessageType::Prune { password } => {
                        if Ok(password) == env::var("LD47_PASSWORD") {
                            self.data.send(server::MessagePrune {
                                addr: ctx.address().recipient(),
                            })
                            .into_actor(self)
                            .then(|res, act, ctx| {
                                match res {
                                    Ok(res) => act.id = res as usize,
                                    // something is wrong with server
                                    _ => ctx.stop(),
                                }
                                fut::ready(())
                            })
                            .wait(ctx);
                        } else {
                            let _ = ctx.address().recipient().do_send(
                                server::ToUserMessage(String::from("Wrong password"))
                            );
                        }
                    },
                    MessageType::FullClean { password } => {
                        if Ok(password) == env::var("LD47_PASSWORD") {
                            self.data.send(server::MessageFullClean {
                                addr: ctx.address().recipient(),
                            })
                            .into_actor(self)
                            .then(|res, act, ctx| {
                                match res {
                                    Ok(res) => act.id = res as usize,
                                    // something is wrong with server
                                    _ => ctx.stop(),
                                }
                                fut::ready(())
                            })
                            .wait(ctx);
                        } else {
                            let _ = ctx.address().recipient().do_send(
                                server::ToUserMessage(String::from("Wrong password"))
                            );
                        }
                    },
                    _ => ()
                }
            },
            Ok(ws::Message::Binary(bin)) => ctx.binary(bin),
            _ => (),
        }
    }
}

async fn index(
    req: HttpRequest, stream: web::Payload, data: web::Data<Addr<server::GameServer>>,
) -> Result<HttpResponse, Error> {
    let resp = ws::start(GameWebSocket {
        id: 0,
        data: data.get_ref().clone(),
    }, &req, stream);
    println!("{:?}", resp);
    resp
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    let server = server::GameServer::new().start();

    if Ok(String::from("SSL")) == env::var("MODE") {
        let mut builder = SslAcceptor::mozilla_intermediate(SslMethod::tls()).unwrap();
        builder
            .set_private_key_file("key.pem", SslFiletype::PEM)
            .unwrap();
        builder.set_certificate_chain_file("cert.pem").unwrap();

        HttpServer::new(move || {
            App::new()
            .data(server.clone())
            .route("/", web::get().to(index))
        })
            .bind_openssl("0.0.0.0:443", builder)?
            .run()
            .await
    } else {
        HttpServer::new(move || {
            App::new()
            .data(server.clone())
            .route("/", web::get().to(index))
        })
            .bind("0.0.0.0:8080")?
            .run()
            .await
    }
}
