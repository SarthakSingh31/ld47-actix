CREATE TABLE game (
    id SERIAL PRIMARY KEY NOT NULL,
    game_started BOOLEAN DEFAULT FALSE NOT NULL
);

CREATE TABLE turn (
    id SERIAL PRIMARY KEY NOT NULL,
    game_id INTEGER REFERENCES game (id) NOT NULL
);

CREATE TABLE player (
    id SERIAL PRIMARY KEY NOT NULL,
    private_key TEXT NOT NULL,
    username TEXT NOT NULL,
    character_type INTEGER NOT NULL,
    is_ai BOOLEAN NOT NULL,
    game_id INTEGER REFERENCES game (id) NOT NULL
);

CREATE TABLE mutation (
    id SERIAL PRIMARY KEY NOT NULL,
    card_type INTEGER NOT NULL,
    card_location INTEGER NOT NULL,
    turn_id INTEGER REFERENCES turn (id) NOT NULL
);

CREATE TABLE card_options_table (
    id SERIAL PRIMARY KEY NOT NULL,
    card_options INTEGER[] NOT NULL,
    player_id INTEGER REFERENCES player (id) NOT NULL,
    turn_id INTEGER REFERENCES turn (id) NOT NULL
);