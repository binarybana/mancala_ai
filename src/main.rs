use std::collections::HashMap;
use std::fmt::{self, Formatter, Display};

#[derive(Debug, Eq, PartialEq, Hash)]
struct GameState {
    houses: [u8; 12],
    ezone1: u8,
    ezone2: u8,
    turn: u8,
    move_counter: u32
}

impl GameState {
    fn new(starting_seeds: u8) -> GameState {
        GameState{ houses: [starting_seeds; 12],
                   ezone1: 0,
                   ezone2: 0,
                   turn: 0,
                   move_counter: 0 }
    }
}

impl Display for GameState {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        // upper row
        try!(write!(f, "+-------------------------------+\n\
                   |   |"));
        // player 1
        for house in &self.houses[..6] {
            try!(write!(f, " {} |", house));
        }

        // end zones
        try!(write!(f, "   |\n| {} |                       | {} |\n\
                   |   |", self.ezone1, self.ezone2));

        // player 2
        for house in &self.houses[6..] {
            try!(write!(f, " {} |", house));
        }

        // last line
        write!(f, "   |\n+-------------------------------+\n")
    }
}
    
fn main() {
    println!("Hello, mancala!");
    let state = GameState::new(4);
    println!("Initial board state: \n{}", state);
    let mut value_fun: HashMap<GameState, f64> = HashMap::with_capacity(1_000_000);
    value_fun.insert(state, 1.3);
    println!("{}", value_fun.len());
}
