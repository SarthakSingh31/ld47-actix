// use std::collections::{HashMap, HashSet};

// use crate::config::*;
// use crate::models::{Game, Player, Mutation};
// use crate::server::GameServer;

// pub struct Simulator;

// impl Simulator {
//     /// Returns a hashmap of user_id to their Animations
//     pub fn sim_turn(game: &mut Game, turn_id: usize) -> HashMap<usize, Vec<Animation>> {
//         let anims = HashMap::new();

//         // Filter out players that have been killed or something
//         let players: Vec<_> = game.players.iter().enumerate().filter(|(_, p)| p.active).collect();
        
//         // Initiate the current state of the board
//         let mut game_board_map = vec![vec![Option::<usize>::None; game.board_size.1 as usize]; game.board_size.0 as usize];
//         for (i, player) in players {
//             game_board_map[player.pos.0 as usize][player.pos.1 as usize] = Some(i);
//         }

//         for tick in 0 .. turn_id {
//             let mut backstack: Vec<usize> = Vec::new();
            
//         }

//         return anims;
//     }
// }