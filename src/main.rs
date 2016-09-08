use std::collections::HashMap;
use std::fmt::{self, Formatter, Display};

#[derive(Debug, Eq, PartialEq, Hash, Copy, Clone)]
pub struct GameState {
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

    fn is_action_valid(&self, action: Action) -> bool {
        self.houses[action as usize] > 0
    }

    fn evaluate_action(&self, action: Action) -> GameState {
        let n = self.houses.len();
        let action = action as usize;
        let seeds = self.houses[action] as usize;
        let mut new_state = self.clone(); //copy
        new_state.houses[action] = 0;
        for i in action+1..action+seeds+1 {
            if i < 6 {
                new_state.houses[i%n] += 1;
            } else if i == 6 { 
                new_state.ezone2 += 1;
                // TODO handle other endzone
            } else {
                // TODO instead of subtracting one, it needs to be the number of times we've wrapped
                // through an ezone div(i,6)?
                new_state.houses[(i-1)%n] += 1;
            }
        }
        // TODO need to implement capture rule here
        new_state
    }

    fn next_valid_move(&self) -> Option<Action> {
        for house in self.houses[0..6] {
            if self.houses[house] > 0 {
                Some(house)
            }
        }
        None
    }


    fn gen_actions(&self) -> ActionIter {
        ActionIter{ next_action: 0 as Action, state: *self }
    }

    fn pick_action(self, values: &ValueFunction) -> Action {
        let choices: Vec<(Action, f64)> = (0..6)
            .filter(|action| self.is_action_valid(*action as Action))
            .map(|action| (action, self.evaluate_action(action)))
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

    /// 'Rotate' the board so player one and two are swapped
    fn swap_board(&mut self) {
        let n = self.houses.len();
        for i in 0..n/2 {
            let temp = self.houses[i];
            self.houses[i] = self.houses[n/2+i];
            self.houses[n/2+i] = temp;
        }
        let temp = self.ezone1;
        self.ezone1 = self.ezone2;
        self.ezone2 = temp;
    }

}

struct ActionIter<'a> {
    next_action: Action,
    state: &'a GameState
}

impl Iterator for ActionIter {
    type Item = Action;
    fn next(&mut self) -> Option<Action> {
        // TODO:
        // use action to update a `copy` of self.state
        // TODO: 
        // check to see if we captured
        // if so, then grab self.state.next_valid_move() and append it to self.next_action
        if self.next_action < 6 {
            self.next_action += 1;
            Some(self.next_action-1)
        } else {
            None
        }
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

pub type ValueFunction = HashMap<GameState, f64>;
pub type Action = u16; 

pub trait ActionTrait {
    fn push_action(self, action: Action) -> Action;
    fn pop_action(&mut self) -> Action;
}

impl ActionTrait for Action {
    fn push_action(self, action: Action) -> Action {
        assert!(action < 7);
        self << 3 | action
    }

    fn pop_action(&mut self) -> Action {
        let popped_action = *self & 7;
        *self >>= 3;
        popped_action
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use std::collections::HashMap;

    #[test]
    fn test_action_iter() {
        let state = GameState::new(4);
        assert_eq!((0..6).collect::<Vec<Action>>(), state.gen_actions().collect::<Vec<Action>>());
    }

    #[test]
    fn test_packed_actions() {
        let action_1: Action = 3;
        assert_eq!(action_1, 3);
        let mut action_2 = action_1.push_action(4);
        println!("{:?}", action_2);
        assert_eq!(action_2.pop_action(), 4);
        assert_eq!(action_2, 3);
    }

    #[test]
    fn pick_actions() {
        let mut value_fun: HashMap<GameState, f64> = HashMap::new();
        let state = GameState::new(4);
        let mut good_state = state.evaluate_action(3);
        value_fun.insert(good_state, 10.0);
        assert_eq!(state.pick_action(&value_fun), 3);
        // Now after swapping the board, it should be a different set of evaluations
        // (ie: our value_fun info will not be useful for any of these particular actions)
        good_state.swap_board();
        let different_state = good_state.evaluate_action(1);
        value_fun.insert(different_state, 4.0);
        assert_eq!(good_state.pick_action(&value_fun), 1);
    }

    #[test]
    fn test_swap_board() {
        let state = GameState::new(4);
        let mut state = state.evaluate_action(4);
        assert_eq!(state.houses[4], 0);
        assert_eq!(state.houses[5], 5);
        state.swap_board();
        println!("{}", state);
        assert_eq!(state.houses[10], 0);
        assert_eq!(state.houses[11], 5);
    }
}

fn sarsa_loop(values: &mut HashMap<GameState, f64>,
              // learning_rate: f64,
              // discount_factor: f64,
              episodes: usize) {
    for i in 0..episodes {
        let state = GameState::new(4);
        let action = state.pick_action(values);
        loop {
            // TODO implement SARSA reward with discounts
            if action == i as Action {
                break;
            }
            break;
            // state.swap_board();
        }
    }
}
    
fn main() {
    println!("Hello, mancala!");
    let state = GameState::new(4);
    println!("Initial board state: \n{}", state);
    let mut state = state.evaluate_action(4);
    println!("Played a 2: \n{}", state);
    state.swap_board();
    println!("Player 2's turn: \n{}", state);
    println!("Has the game ended? {}", state.is_ended());
    let mut value_fun: HashMap<GameState, f64> = HashMap::with_capacity(1_000);
    value_fun.insert(state, 1.3);
    sarsa_loop(&mut value_fun, 1_000);
    // sarsa_loop(&mut value_fun, 0.1, 0.1, 1_000);
    println!("Number of values in our state value table/map: {}", value_fun.len());
}
