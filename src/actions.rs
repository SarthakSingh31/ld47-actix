use diesel::prelude::*;
use crate::models::{Game, Turn};

pub fn get_current_turn_of_game(game: &Game, conn: &PgConnection) -> Option<Turn> {
    get_current_turn_of_game_id(game.id, conn)
}

pub fn get_current_turn_of_game_id(game_id_val: i32, conn: &PgConnection) -> Option<Turn> {
    use crate::schema::turn::dsl::{game_id, turn};

    match turn.filter(game_id.eq(game_id_val)).load::<Turn>(conn) {
        Ok(mut turns) => turns.pop(),
        Err(_) => None
    }
}