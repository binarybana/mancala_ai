use std::collections::HashMap;
use std::fmt::{self, Formatter, Display};

#[derive(Debug, Eq, PartialEq, Hash, Copy, Clone)]
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

        // player 2
        for house in self.houses[6..].iter().rev() {
            try!(write!(f, " {} |", house));
        }

        // end zones
        try!(write!(f, "   |\n| {} |                       | {} |\n\
                   |   |", self.ezone1, self.ezone2));

        // player 1
        for house in &self.houses[..6] {
            try!(write!(f, " {} |", house));
        }

        // last line
        write!(f, "   |\n+-------------------------------+\n")
    }
}

type Action = u16; 
type ValueFunction = HashMap<GameState, f64>;

fn is_action_valid(state: &GameState, action: Action) -> bool {
    state.houses[action as usize] > 0
}

fn push_action(action_list: Action, action: Action) -> Action {
    assert!(action < 7);
    action_list << 3 | action
}

fn pop_action(action: &mut Action) -> Action {
    let popped_action = *action & 7;
    *action >>= 3;
    popped_action
}

#[test]
fn test_packed_actions() {
    let action_1: Action = 3;
    assert_eq!(action_1, 3);
    let mut action_2 = push_action(action_1, 4);
    println!("{:?}", action_2);
    assert_eq!(pop_action(&mut action_2), 4);
    assert_eq!(action_2, 3);
}

fn evaluate_action(state: &GameState, action: Action) -> GameState {
    let N = state.houses.len();
    let action = action as usize;
    let seeds = state.houses[action] as usize;
    let mut new_state = state.clone(); //copy
    new_state.houses[action] = 0;
    for i in action+1..action+seeds+1 {
        if i < 6 {
            new_state.houses[i%N] += 1;
        } else if i == 6 { 
            new_state.ezone2 += 1;
            // TODO handle other endzone
        } else {
            // TODO instead of subtracting one, it needs to be the number of times we've wrapped
            // through an ezone div(i,6)?
            new_state.houses[(i-1)%N] += 1;
        }
    }
    // TODO need to implement capture rule here
    new_state
}

fn pick_action(state: GameState, values: &ValueFunction) -> Action {
    let choices: Vec<(Action, f64)> = (0..6)
        .filter(|action| is_action_valid(&state, *action as Action))
        .map(|action| (action, evaluate_action(&state, action)))
        .map(|(action, possible_state)| (action, *values.get(&possible_state).unwrap_or(&0.1f64)))
        .collect();
    let mut best = &choices[0];
    for choice in &choices {
        if choice.1 > best.1 {
            best = choice;
        }
    }
    best.0 // return the best action
}

#[test]
fn pick_actions() {
    let mut value_fun: HashMap<GameState, f64> = HashMap::new();
    let mut state = GameState::new(4);
    let mut good_state = evaluate_action(&state, 3);
    value_fun.insert(good_state, 10.0);
    assert_eq!(pick_action(state, &value_fun), 3);
    // Now after swapping the board, it should be a different set of evaluations
    // (ie: our value_fun info will not be useful for any of these particular actions)
    swap_board(&mut good_state);
    let different_state = evaluate_action(&good_state, 1);
    value_fun.insert(different_state, 4.0);
    assert_eq!(pick_action(good_state, &value_fun), 1);
}

/// 'Rotate' the board so player one and two are swapped
fn swap_board(state: &mut GameState) {
    let N = state.houses.len();
    for i in 0..N/2 {
        let temp = state.houses[i];
        state.houses[i] = state.houses[N/2+i];
        state.houses[N/2+i] = temp;
    }
    let temp = state.ezone1;
    state.ezone1 = state.ezone2;
    state.ezone2 = temp;
}

#[test]
fn test_swap_board() {
    let state = GameState::new(4);
    let mut state = evaluate_action(&state, 4);
    assert_eq!(state.houses[4], 0);
    assert_eq!(state.houses[5], 5);
    swap_board(&mut state);
    println!("{}", state);
    assert_eq!(state.houses[10], 0);
    assert_eq!(state.houses[11], 5);
}

fn sarsa_loop(values: &mut HashMap<GameState, f64>,
              learning_rate: f64,
              discount_factor: f64,
              episodes: usize) {
    for i in 0..episodes {
        let mut state = GameState::new(4);
        let action = pick_action(state, values);
        loop {
            // TODO implement SARSA reward with discounts
            if action == 1 {
                break;
            }
            break;
            // swap_board(&mut state);
        }
    }
}
    
fn main() {
    println!("Hello, mancala!");
    let state = GameState::new(4);
    println!("Initial board state: \n{}", state);
    let mut state = evaluate_action(&state, 4);
    println!("Played a 2: \n{}", state);
    swap_board(&mut state);
    println!("Player 2's turn: \n{}", state);
    println!("Has the game ended? {}", state.is_ended());
    let mut value_fun: HashMap<GameState, f64> = HashMap::with_capacity(1_000);
    value_fun.insert(state, 1.3);
    sarsa_loop(&mut value_fun, 0.1, 0.1, 1_000);
    println!("Number of values in our state value table/map: {}", value_fun.len());
}
