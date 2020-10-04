table! {
    card_options_table (id) {
        id -> Int4,
        card_options -> Array<Int4>,
        player_id -> Int4,
        turn_id -> Int4,
    }
}

table! {
    game (id) {
        id -> Int4,
        game_started -> Bool,
    }
}

table! {
    mutation (id) {
        id -> Int4,
        card_type -> Int4,
        card_location -> Int4,
        turn_id -> Int4,
    }
}

table! {
    player (id) {
        id -> Int4,
        private_key -> Text,
        username -> Text,
        character_type -> Int4,
        is_ai -> Bool,
        game_id -> Int4,
    }
}

table! {
    turn (id) {
        id -> Int4,
        game_id -> Int4,
    }
}

joinable!(card_options_table -> player (player_id));
joinable!(card_options_table -> turn (turn_id));
joinable!(mutation -> turn (turn_id));
joinable!(player -> game (game_id));
joinable!(turn -> game (game_id));

allow_tables_to_appear_in_same_query!(
    card_options_table,
    game,
    mutation,
    player,
    turn,
);
