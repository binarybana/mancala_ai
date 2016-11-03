// #![feature(proc_macro)]
// #[macro_use]
// extern crate serde_derive;
// extern crate serde_json;
// extern crate clap;
// use clap::{Arg, App, SubCommand, AppSettings};

extern crate rustc_serialize;
extern crate docopt;

use docopt::Docopt;

const USAGE: &'static str = "
Mancala AI using reinforcement learning.

Usage:
  mancala [--num-runs=<num-runs>] [--learning-rate=<a>] [--discount-rate=<g>] [--epsilon=<epsilon>]
  mancala (-h | --help)
  mancala --version

Options:
  -h --help              Show this screen.
  --version              Show version.
  --num-runs=<num-runs>  Number of complete games [default: 10].
  --epsilon=<epsilon>    Epsilon for non-greedy actions [default: 0.02].
  --learning-rate=<a>    Learning rate [default: 0.05].
  --discount-rate=<g>    Discount rate [default: 1.0].
";

#[derive(Debug, RustcDecodable)]
struct Args {
    flag_num_runs: usize,
    flag_epsilon: f64,
    flag_learning_rate: f64,
    flag_discount_rate: f64,
}


#[macro_use]
extern crate log;
extern crate env_logger;
extern crate rand;
use rand::Rng;

use std::collections::HashMap;
use std::fmt::{self, Formatter, Display};

mod packed_actions;
use packed_actions::{Action, SubAction, ActionQueue};

// #[derive(Serialize, Deserialize, Debug,
#[derive(Debug,
         Eq, PartialEq, Hash, Copy, Clone)]
pub struct GameState {
    houses: [u8; 14],
}

impl GameState {
    /// Create a new board initialized with each house having `starting_seeds` number of seeds.
    fn new(starting_seeds: u8) -> GameState {
        let mut state = GameState{ houses: [starting_seeds; 14] };
        state.houses[6] = 0;
        state.houses[13] = 0;
        state
    }
    
    /// Is the game completely over where one player has emptied their side of the board?
    fn is_ended(&self) -> bool {
        let p1_tot: u8 = self.houses[..6].iter().sum();
        let p2_tot: u8 = self.houses[7..13].iter().sum();
        if p1_tot == 0 || p2_tot == 0 {
            return true;
            info!("Checking if game is ended: Yes!");
        }
        info!("Checking if game is ended: no... {}, {}", p1_tot, p2_tot);
        return false;
    }

    /// Return a new game state when playing out a sequence of actions (a string of capturing
    /// moves)
    fn evaluate_to_new_state(&self, action_list: Action) -> GameState {
        let mut new_state = self.clone();
        new_state.evaluate_action(action_list);
        new_state
    }

    /// Mutate the current game state when playing out a full action sequence
    fn evaluate_action(&mut self, mut action_list: Action) {
        // TODO: make this a proper iterator
        // for each action in action_list
        loop {
            let subaction = action_list.pop_action();
            self.evaluate_subaction(subaction);
            if action_list.is_empty() { break; }
        }
    }

    /// Mutate the current game state when playing out a single subaction
    fn evaluate_subaction(&mut self, subaction: SubAction) {
        let action = subaction as usize;
        assert!(action != 6 && action != 13);
        let seeds = self.houses[action] as usize;
        // Pickup seeds from starting house
        self.houses[action] = 0;
        let end_house = action+seeds % 14;
        // Deposit seeds in each house around the board
        for i in action+1..end_house+1 {
            self.houses[i%14] += 1;
        }
        // Capture rule
        if end_house < 6 && self.houses[end_house] == 1 {
            // add to capture pile
            let opposing_house = 12 - end_house;
            self.houses[6] += 1 + self.houses[opposing_house];
            // clear houses on both sides
            self.houses[end_house] = 0;
            self.houses[opposing_house] = 0;
            info!("Capture detected!");
        }
    }

    // fn next_valid_submove(&self) -> Option<SubAction> {
    //     for house in &self.houses[0..6] {
    //         if self.houses[*house as usize] > 0 {
    //             return Some(*house as SubAction);
    //         }
    //     }
    //     return None;
    // }

    fn gen_actions(&self) -> ActionIter {
        ActionIter{ next_subaction: 0, state: &self }
    }

    fn pick_action(self, epsilon: f64, values: &ValueFunction) -> (Action, f64) {
        let choices: Vec<(Action, f64)> = self.gen_actions()
            .map(|action| (action, self.evaluate_to_new_state(action)))
            .map(|(action, possible_state)| (action, *values.get(&possible_state)
                                             .unwrap_or(&0.5f64)))
            .collect();
        info!("Actions available to choose from: {:?}", choices);
        if choices.len() == 0 {
            println!("state: {}", self);
        }
        assert!(choices.len() > 0);
        let mut best = &choices[0];
        if rand::random::<f64>() < epsilon {
            // randomly make a move
            best = rand::thread_rng().choose(&choices).unwrap();
         } else {
            for choice in &choices {
                if choice.1 > best.1 {
                    best = choice;
                }
            }
        }
        best.clone()
    }

    /// 'Rotate' the board so player one and two are swapped
    fn swap_board(&mut self) {
        let n = self.houses.len();
        for i in 0..n/2 {
            let temp = self.houses[i];
            self.houses[i] = self.houses[n/2+i];
            self.houses[n/2+i] = temp;
        }
    }
}

struct ActionIter<'a> {
    next_subaction: SubAction,
    state: &'a GameState
}

impl<'a> Iterator for ActionIter<'a> {
    type Item = Action;
    fn next(&mut self) -> Option<Action> {
        // TODO: this is without multiple subturns when capturing
        let mut action = Action::new();
        for index in self.next_subaction..6 {
            if self.state.houses[index as usize] > 0 {
                info!("Pushing subaction of {} because there are {} seeds there",
                      index, self.state.houses[index as usize]);
                action.push_action(index);
                self.next_subaction = index + 1;
                return Some(action);
            }
        }
        return None;

        // An early attempt at the full capturing, multiple sub-turn dynamics:
        // if captured
        // if !self.action.is_empty() {
        //     // find next subaction and return
        //     // or pop and keep searching?
        //     self.action.push_action(self.next_valid_submove)
        //     // }
        // } else {
        //     something
        //
        // }
        // // TODO:
        // // use action to update a `copy` of self.state
        // // TODO: 
        // // check to see if we captured
        // // if so, then grab self.state.next_valid_move() and append it to self.next_action
        // if self.next_action < 6 {
        //     self.next_action += 1;
        //     Some(self.next_action-1)
        // } else {
        //     None
        // }
    }
}

impl Display for GameState {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        // upper row
        try!(write!(f, "+-------------------------------+\n\
                   |   |"));
        // player 2
        for house in self.houses[7..13].iter().rev() {
            try!(write!(f, "{:2} |", house));
        }
        // end zones
        try!(write!(f, "   |\n|{:2} |                       |{:2} |\n\
                   |   |", self.houses[13], self.houses[6]));
        // player 1
        for house in &self.houses[0..6] {
            try!(write!(f, "{:2} |", house));
        }
        // last line
        write!(f, "   |\n+-------------------------------+\n")
    }
}

pub type ValueFunction = HashMap<GameState, f64>;

#[cfg(test)]
mod test {
    use super::*;
    use packed_actions::*;
    use std::collections::HashMap;

    #[test]
    fn test_action_iter() {
        let state = GameState::new(4);
        let mut action = Action::new();
        for (subaction, state_action) in (0..6).zip(state.gen_actions()) {
            action.push_action(subaction);
            assert_eq!(action, state_action);
            action.pop_action();
        }
    }

    #[test]
    fn pick_actions() {
        let mut value_fun: HashMap<GameState, f64> = HashMap::new();
        let mut state = GameState::new(4);
        let action = Action::singleton(3);
        let mut good_state = state.clone();
        good_state.evaluate_action(action);
        value_fun.insert(good_state, 10.0);
        assert_eq!(state.pick_action(0.0, &value_fun).0, action);
        // Now after performing that option and swapping the board, it should be a 
        // different set of evaluations (ie: our value_fun info will not be useful 
        // for any of these particular actions)
        state.evaluate_action(action);
        state.swap_board();
        let mut p2_good_state = state.clone();
        p2_good_state.evaluate_action(Action::singleton(1));
        value_fun.insert(p2_good_state, 4.0);
        println!("{:?}", state.pick_action(0.0, &value_fun));
        assert_eq!(state.pick_action(0.0, &value_fun).0, Action::singleton(1));

        let mut mut_flag = false;
        for _ in 0..10 {
            if state.pick_action(1.0, &value_fun).0 != Action::singleton(1) {
                mut_flag = true;
            }
        }
        assert_eq!(mut_flag, true);
    }

    #[test]
    fn test_swap_board() {
        let mut state = GameState::new(4);
        let action = Action::singleton(4);
        state.evaluate_action(action);
        assert_eq!(state.houses[4], 0);
        assert_eq!(state.houses[5], 5);
        state.swap_board();
        println!("{}", state);
        assert_eq!(state.houses[11], 0);
        assert_eq!(state.houses[12], 5);
    }
}

fn sarsa_loop(values: &mut HashMap<GameState, f64>,
              epsilon: f64,
              learning_rate: f64,
              discount_factor: f64,
              episodes: usize) {
    let default_state_val = 0.5f64;
    let mut q_prev = 0.0;
    let mut q_next = 0.0;
    let mut action = Action::new();
    
    for _ in 0..episodes {
        let mut last_p1_state = GameState::new(4);
        let mut last_p2_state = GameState::new(4);
        let mut last_state = GameState::new(4);
        let mut state = GameState::new(4);
        info!("");
        info!("");
        info!("######################");
        info!("######################");
        info!(">>>>>>>>>>>>>>>>>");
        let mut counter = 0;
        loop {
            info!("Turn {}", counter);
            last_state = if counter % 2 == 0 { last_p1_state } else { last_p2_state };
            {
                q_prev = *values.get(&last_state).unwrap_or(&default_state_val);
                let tup = state.pick_action(epsilon, values);
                action = tup.0;
                q_next = tup.1;
            }
            let score_diff = state.houses[6] as f64 - state.houses[13] as f64;
            info!("State: \n{}", state);
            info!("Action: {}", action);
            state.evaluate_action(action);
            // let reward = (state.houses[6] as f64 - state.houses[13] as f64) - score_diff;
            // FIXME: noop
            let reward = if state.is_ended() { 0 } else { 0 };
            info!("Reward: {}, score_diff: {}", reward, score_diff);
            {
                let q_ref = values.entry(state).or_insert(default_state_val);
                *q_ref += learning_rate * (reward as f64 + discount_factor * q_next - q_prev);
                info!("q_ref += learning_rate * (reward + discount_factor * q_next - q_prev)\n\
                    {} += {} * ({} + {} * {} - {})",
                    *q_ref, learning_rate, reward, discount_factor, q_next, q_prev);
            }
            if state.is_ended() {

                info!("Game ended at state:");
                info!("{}", state);
                counter += 1;
                if counter % 2 == 0 {
                    // Player 2's turn just finished
                    *values.entry(last_p2_state).or_insert(default_state_val) = 1.0;
                    *values.entry(last_p1_state).or_insert(default_state_val) = -1.0;
                } else {
                    *values.entry(last_p1_state).or_insert(default_state_val) = 1.0;
                    *values.entry(last_p2_state).or_insert(default_state_val) = -1.0;
                }
                break;
            }
            counter += 1;
            if counter % 2 == 0 {
                // Player 2's turn just finished
                last_p2_state = state.clone();
            } else {
                last_p1_state = state.clone();
            }
            state.swap_board();
            info!(">>>>>>>>>>>>>>>>>");
        }
    }
}
    
fn main() {
    env_logger::init().unwrap();
    info!("Hello, mancala!");
    let args: Args = Docopt::new(USAGE)
                            .and_then(|d| d.decode())
                            .unwrap_or_else(|e| e.exit());


    let mut value_fun: HashMap<GameState, f64> = HashMap::with_capacity(1_000);
    sarsa_loop(&mut value_fun,
               args.flag_epsilon,
               args.flag_learning_rate,
               args.flag_discount_rate,
               args.flag_num_runs);

    let mut qvals: Vec<_> = value_fun.values().collect();
    qvals.sort_by(|a, b| b.partial_cmp(a).unwrap());
    println!("Some top values from value_fun:");
    for val in &qvals[..10] {
        println!("\t{}", val);
    }
    println!("Number of entries in value function: {}", value_fun.len());

    let mut vals = value_fun.iter().collect::<Vec<_>>();
    vals.sort_by(|a, b| b.1.partial_cmp(a.1).unwrap());
    println!("Here's a few of the top values and states:");
    for pair in vals.iter().take(5) {
        println!("\n#########\n{}:\n", pair.1);
        println!("{}", pair.0);
        // let serialized = serde_json::to_string(&vals[..5].to_vec()).unwrap();
        // println!("serialized = {}", serialized);
    }

}
