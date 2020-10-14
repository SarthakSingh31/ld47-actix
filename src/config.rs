pub struct Animation {
    pub move_distance: u16,
    pub rotation: u8,
}

pub struct Card {
    pub disorient: bool,
    pub dmg: u8,
    pub actions: Vec<usize>
}

pub struct GameConfig {
    pub cards: usize,
    pub actions: usize,
}

impl GameConfig {
    pub fn default() -> Self {
        GameConfig {
            cards: 38,
            actions: 0,
        }
    }
}

