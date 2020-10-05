pub struct PlayerAction {
    pub move_distance: u16,
    pub rotation: u8,
}

pub struct Card {
    pub disorient: bool,
    pub dmg: u8,
    pub actions: Vec<usize>
}

pub struct GameConfig {
    pub cards: Vec<Card>,
    pub actions: Vec<PlayerAction>,
}

impl GameConfig {
    pub fn default() -> Self {
        let actions = vec![
            PlayerAction {
                move_distance: 1,
                rotation: 0,
            }
        ];

        let cards = vec! [
            Card {
                disorient: false,
                dmg: 0,
                actions: vec![0]
            },
            Card {
                disorient: true,
                dmg: 5,
                actions: vec![0, 0, 0]
            }
        ];
        GameConfig {
            cards: cards,
            actions: actions,
        }
    }
}

