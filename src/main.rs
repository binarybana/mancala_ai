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
    
    fn is_ended(&self) -> bool {
        let p1_tot: u8 = self.houses[..6].iter().fold(0, std::ops::Add::add);
        let p2_tot: u8 = self.houses[6..].iter().fold(0, std::ops::Add::add);
        if p1_tot == 0 || p2_tot == 0 {
            return true;
        }
        return false;
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

type Action = u8; 
type ValueFunction = HashMap<GameState, f64>;

fn pick_action(state: GameState, values: &ValueFunction) -> Action {
    if turn == 0 {
        let choices = (0..6)
            .filter(|action| action_valid(state, action))
            .map(|action| (action, evaluate_action(state, action)))
            .map(|(action, state)| (action, values.get(state).unwrap_or(0.1)));
        let mut best = &choices[0];
        for choice in &choices {
            if choice.1 > best.1 {
                best = choice;
            }
        }
        return best
        }
    else {

}

fn sarsa_loop(values: &mut HashMap<GameState, f64>,
              learning_rate: f64,
              discount_factor: f64,
              episodes: usize) {
    for i in 0..episodes {
        let state = GameState::new(4);
        let action = pick_action(state);
        loop {
            if action == 1 {
                break
            }
        }
    }
}

    
fn main() {
    println!("Hello, mancala!");
    let state = GameState::new(4);
    println!("Initial board state: \n{}", state);
    println!("Has the game ended? {}", state.is_ended());
    let mut value_fun: HashMap<GameState, f64> = HashMap::with_capacity(1_000_000);
    value_fun.insert(state, 1.3);
    sarsa_loop(&mut value_fun, 0.1, 0.1, 1);
    println!("{}", value_fun.len());
}
