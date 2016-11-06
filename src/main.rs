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
        }
        info!("Checking if game is ended: no... {}, {}", p1_tot, p2_tot);
        return false;
    }

    /// Move other players seeds to their house after a game ends
    fn finalize_game(&mut self) {
        for i in 7..13 {
            self.houses[13] += self.houses[i];
            self.houses[i] = 0;
        }
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
        // Offset is to handle skipping of the opponents
        // scoring house as we go around the loop
        let mut offset = 0;
        for i in action+1..end_house+1 {
            if i > 0 && i % 13 == 0 {
                self.houses[0] += 1;
                offset += 1;
            } else {
                self.houses[(i+offset)%14] += 1;
            }
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
        ActionIter{ next_action: Action::new(),
                    base_state: &self,
                    state_stack: Vec::new()
                  }
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

    fn find_next_subaction(&self, search_start: Subaction) -> Option<SubAction> {
        for index in search_start..6 {
            if self.houses[index as usize] > 0 {
                Some(index)
            }
        }
        None
    }
}

struct ActionIter<'a> {
    next_action: Action,
    base_state: &'a GameState,
    state_stack: Vec<GameState>,
}

impl ActionIter<'a> {
    /// Assuming that this ActionIter has been fully initialized, find the 
    /// next action to take
    fn set_next_action(&mut self) {
        // I think this should be working correctly
        loop {
            let curr_state = if state_stack.is_empty() { base_state } 
                             else { state_stack[state_stack.length()-1] };
            let prev_subaction = self.next_action.pop_action();
            if let Some(sub) = curr_state.find_next_subaction(prev_subaction+1) {
                self.next_action.push_action(sub);
                if state.is_capture_subaction(sub) { // need to make sure we explore this state more
                    self.state_stack.push(curr_state.evaluate_to_new_state(sub));
                } 
                break; // we found a valid subaction for this curr_state
            } else { // no more subactions in this state
                if state_stack.is_empty() { // all done
                    assert(self.next_action.is_empty());
                    break;
                } else { // try the next parent subaction
                    state_stack.pop();
                }
            }
        }
    }
}

impl<'a> Iterator for ActionIter<'a> {
    type Item = Action;
    fn next(&mut self) -> Option<Action> {
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
        /////////////////////
        /////////////////////
        /////////////////////

        //  TODO THIS IS ALL BUSTED
        // Full capturing with multiple sub-turn dynamics:
        if self.next_action.is_empty() {
            // find next two actions and return the first and store the 
            // second in self.next_action
            let mut curr_action = Action::new();
            for index in 0..6 {
                let seeds = self.base_state.houses[index];
                if seeds > 0 {
                    if curr_action.is_empty() {
                        curr_action.push_action(index as u8);
                        if seeds + index as u8 == 6 { // multi turn
                            let new_state = self.base_state.evaluate_to_new_state(curr_action);
                            self.state_stack.push(new_state);
                            match new_state.find_next_subaction() {
                                Some(sub) => something,
                                None => something
                            }

                        } else { // single turn
                        }
                    }
                }
            }
        } else { // iterator already setup
            if self.next_action.is_empty() {
                return None
            }
            let ret_val = self.next_action;
            if self.next_action.is_some() {
                self.set_next_action();
            }
            return Some(ret_val);
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
    fn test_evaluate_actions() {
        let mut state = GameState::new(0);
        let action = Action::singleton(4);
        state.houses[4] = 10;
        state.evaluate_action(action);
        for i in 5..13 {
            assert_eq!(state.houses[i], 1);
        }
        assert_eq!(state.houses[13], 0);
        assert_eq!(state.houses[0], 1);
        assert_eq!(state.houses[1], 1);
        assert_eq!(state.houses[2], 0);
        assert_eq!(state.houses[3], 0);
        assert_eq!(state.houses[4], 0);
    }

    #[test]
    fn test_capture_rules() {
        let mut state = GameState::new(4);
        state.houses[4] = 0;
        let action = Action::singleton(0);
        state.evaluate_action(action);
        let expected: [u8; 14] = [0,5,5,5,0,4, 5, 4,0,4,4,4,4, 0];
        assert_eq!(state.houses, expected);
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
    fn test_finalize_game() {
        let mut state = GameState::new(4);
        state.finalize_game();
        assert_eq!(state.houses[13], 4*6);
        for i in 7..13 {
            assert_eq!(state.houses[i], 0);
        }
        for i in 0..6 {
            assert_eq!(state.houses[i], 4);
        }
        assert_eq!(state.houses[6], 0);
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

fn dump_counter_stats(lens: &Vec<usize>, header_only: bool) {

    let buckets = vec![0,5,10,15,20,25,30,45,50,60,70,80,90,100];
    if header_only {
        for buc in buckets.iter() {
            print!("[{:5}] ", buc);
        }
        println!("");
        return;
    }
    let mut counts = Vec::new();

    for i in 1..buckets.len() {
        let count = lens.iter()
                        .filter(|val| **val < buckets[i] && **val >= buckets[i-1])
                        .count();
        counts.push(count);
    }
    for count in counts.iter() {
        print!("{:7} ", count);
    }
    println!("");
}

fn sarsa_loop(values: &mut HashMap<GameState, f64>,
              epsilon: f64,
              learning_rate: f64,
              discount_factor: f64,
              episodes: usize) {
    let default_state_val = 0.5f64;
    let mut q_prev: f64;
    let mut q_next: f64;
    let mut action: Action;
    let mut game_lengths = Vec::with_capacity(episodes);
    dump_counter_stats(&game_lengths, true);
    
    for episode in 0..episodes {
        let mut last_p1_state = GameState::new(4);
        let mut last_p2_state = GameState::new(4);
        let mut state = GameState::new(4);
        info!("");
        info!("");
        info!("######################");
        info!("######################");
        info!(">>>>>>>>>>>>>>>>>");
        let mut counter = 0;
        loop {
            let players_turn = if counter % 2 == 0 { 1 } else { 2 };
            let last_state = if players_turn == 1 { last_p1_state } else { last_p2_state };
            info!("Turn {}, player {}'s turn", counter, players_turn);
            {
                q_prev = *values.get(&last_state).unwrap_or(&default_state_val);
                if players_turn == 2 {
                    let tup = state.pick_action(epsilon, values);
                    action = tup.0;
                    q_next = tup.1;
                } else {
                    let tup = state.pick_action(epsilon, values);
                    action = tup.0;
                    q_next = tup.1;
                }
                // action = tup.0;
                // q_next = tup.1;
            }
            info!("State before action: \n{}", state);
            info!("Action: {}", action);
            state.evaluate_action(action);
            trace!("State after action: \n{}", state);
            {
                let q_ref = values.entry(last_state).or_insert(default_state_val);
                *q_ref += learning_rate * (discount_factor * q_next - q_prev);
                info!("q_ref += learning_rate * (discount_factor * q_next - q_prev)\n\
                    {} += {} * ({} * {} - {})",
                    *q_ref, learning_rate, discount_factor, q_next, q_prev);
            }
            if state.is_ended() {
                info!("Game ended at state:\n{}", state);
                state.finalize_game();
                let curr_player_win = state.houses[6] > state.houses[13];
                let tie = state.houses[6] == state.houses[13];
                if curr_player_win && players_turn == 1 || !curr_player_win && players_turn == 2 {
                    trace!("P1 win");
                    *values.entry(last_p1_state).or_insert(default_state_val) = 1.0;
                    *values.entry(last_p2_state).or_insert(default_state_val) = -1.0;
                } else if !tie {
                    trace!("P2 win");
                    *values.entry(last_p2_state).or_insert(default_state_val) = 1.0;
                    *values.entry(last_p1_state).or_insert(default_state_val) = -1.0;
                }
                counter += 1;
                game_lengths.push(counter);
                break;
            }
            if players_turn == 1 {
                last_p1_state = state.clone();
            } else {
                last_p2_state = state.clone();
            }
            counter += 1;
            state.swap_board();
            info!(">>>>>>>>>>>>>>>>>");
        }
        if (episode+1) % 10000 == 0 {
            dump_counter_stats(&game_lengths, false);
            game_lengths.clear();
        }
    }
    dump_counter_stats(&game_lengths, false);
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

    // let mut qvals: Vec<_> = value_fun.values().collect();
    // qvals.sort_by(|a, b| b.partial_cmp(a).unwrap());
    // println!("Some top values from value_fun:");
    // for val in &qvals[..10] {
    //     println!("\t{}", val);
    // }
    println!("Number of entries in value function: {}", value_fun.len());

    // let mut vals = value_fun.iter().collect::<Vec<_>>();
    // vals.sort_by(|a, b| b.1.partial_cmp(a.1).unwrap());
    // println!("Here's a few of the top values and states:");
    // for pair in vals.iter().take(5) {
    //     println!("\n#########\n{}:\n", pair.1);
    //     println!("{}", pair.0);
    //     // let serialized = serde_json::to_string(&vals[..5].to_vec()).unwrap();
    //     // println!("serialized = {}", serialized);
    // }

}
