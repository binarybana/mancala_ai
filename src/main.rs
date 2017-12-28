// #![feature(proc_macro)]
// #[macro_use]
// extern crate serde_derive;
extern crate bincode;
extern crate rustc_serialize;

use bincode::SizeLimit;
use bincode::rustc_serialize::{encode, decode};

use std::fs::File;
use std::io::{Read, Write};

// extern crate serde_json;
// extern crate clap;
// use clap::{Arg, App, SubCommand, AppSettings};

extern crate docopt;

use docopt::Docopt;

const USAGE: &'static str = "
Mancala AI using reinforcement learning.

Usage:
  mancala train [--num-runs=<num-runs>] [--learning-rate=<a>] [--discount-rate=<g>] [--epsilon=<epsilon>] [--train=<train>]
  mancala play [--train=<train>]
  mancala (-h | --help)
  mancala --version

Options:
  -h --help              Show this screen.
  --version              Show version.
  --num-runs=<num-runs>  Number of complete games [default: 10].
  --epsilon=<epsilon>    Epsilon for non-greedy actions [default: 0.02].
  --learning-rate=<a>    Learning rate [default: 0.05].
  --discount-rate=<g>    Discount rate [default: 1.0].
  --train=<train>        Output/input training datafile.
";

#[derive(Debug, RustcDecodable)]
struct Args {
    flag_num_runs: usize,
    flag_epsilon: f64,
    flag_learning_rate: f64,
    flag_discount_rate: f64,
    flag_train: Option<String>,
    cmd_train: bool,
    cmd_play: bool,
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

#[derive(Debug, PartialEq)]
pub enum PlayerTurn {
    P1,
    P2,
}

#[derive(Debug, PartialEq)]
pub enum Outcome {
    P1Win,
    P2Win,
    Tie,
}
use Outcome::*;

#[derive(Debug, Eq, PartialEq, Hash, Copy, Clone, RustcDecodable, RustcEncodable)]
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

    /// Is the game a winning final state for current player?
    /// None here means the game is not done.
    fn is_won(&self) -> Option<Outcome> {
        let p1_tot: u8 = self.houses[..6].iter().sum();
        let p2_tot: u8 = self.houses[7..13].iter().sum();
        if p1_tot != 0 && p2_tot != 0 {
            return None;
        }
        let p1_tot = p1_tot + self.houses[6];
        let p2_tot = p2_tot + self.houses[13];
        if p1_tot > p2_tot {
            Some(P1Win)
        } else if p2_tot > p1_tot {
            Some(P2Win)
        } else {
            Some(Tie)
        }
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
            let subaction = action_list.pop_front();
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

    /// Determine if subaction is 'renewing' and grants another turn
    fn is_renewing_subaction(&self, sub: SubAction) -> bool {
        self.houses[sub as usize] + sub == 6
    }

    fn gen_actions(&self) -> ActionIter {
        ActionIter{ action: Action::new(),
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
        info!("Actions available to choose from:");
        for action in &choices {
            info!("\t{}, {}", action.0, action.1);
        }
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

    fn find_next_subaction(&self, search_start: SubAction) -> Option<SubAction> {
        for index in search_start..6 {
            if self.houses[index as usize] > 0 {
                return Some(index)
            }
        }
        None
    }
}

struct ActionIter<'a> {
    action: Action,
    base_state: &'a GameState,
    state_stack: Vec<GameState>,
}

impl<'a> ActionIter<'a> {
    fn get_current_state(&self) -> GameState {
            if self.state_stack.is_empty() {
                self.base_state.clone()
            } else { 
                self.state_stack[self.state_stack.len()-1]
            }
    }

    /// With a valid first_sub SubAction, do a DFS to find a terminal state
    fn next_terminal_state(&mut self, first_sub: SubAction) {
        let mut search = Some(first_sub);
        let mut curr_state = self.get_current_state();
        while let Some(sub) = search {
            trace!("Pushing subaction {} onto action {}", sub, self.action);
            self.action.push_front(sub);
            if curr_state.is_renewing_subaction(sub) {
                trace!("Found renewing sub! pushing state and looking for new subaction");
                curr_state.evaluate_subaction(sub);
                self.state_stack.push(curr_state);
                search = curr_state.find_next_subaction(0);
            } else {
                break;
            }
        }
    }
}

impl<'a> Iterator for ActionIter<'a> {
    type Item = Action;
    fn next(&mut self) -> Option<Action> {
        // if action is empty: find terminal state and return
        // else:
        //   loop {
        //      if subaction: find terminal state at that subaction; break
        //      else: pop_back and pop_state
        //   }
        trace!("Beginning ActionIter.next");
        if self.action.is_empty() { // need to initialize search at terminal state
            if let Some(sub) = self.base_state.find_next_subaction(0) {
                trace!("self.action was empty, initializing search and found next_subaction: {:?}", sub);
                trace!("Now calling `next_terminal_state`");
                self.next_terminal_state(sub);
                return Some(self.action);
            }
            return None;
        }
        loop {
            let curr_state = self.get_current_state();
            trace!("self.action was not empty: {}, finding next subaction", self.action);
            let prev_subaction = self.action.pop_back();
            trace!("base_state: \n{}", self.base_state);
            trace!("curr_state: \n{}", curr_state);
            trace!("popped subaction {} off self.action", prev_subaction);
            if let Some(sub) = curr_state.find_next_subaction(prev_subaction+1) {
                self.next_terminal_state(sub);
                return Some(self.action);
            } else {
                trace!("Couldn't find any subactions at state\n{}", curr_state);
                if self.state_stack.is_empty() || curr_state.is_ended() { // all done
                    trace!("All done");
                    return None;
                } else { // try the next parent subaction
                    trace!("Popping stack");
                    self.state_stack.pop();
                }
            }
        }
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
    extern crate env_logger;

    #[test]
    fn test_action_iter() {
        let _ = env_logger::init();
        let state = GameState::new(4);
        let actions = state.gen_actions().collect::<Vec<_>>();
        assert_eq!(actions.len(), 10);
        // (len: 1; 0,)
        // (len: 1; 1,)
        // (len: 2; 2,0,)
        // (len: 2; 2,1,)
        // (len: 2; 2,3,)
        // (len: 2; 2,4,)
        // (len: 2; 2,5,)
        // (len: 1; 3,)
        // (len: 1; 4,)
        // (len: 1; 5,)

        // setup two stage nested turn
        let mut state = GameState::new(4);
        state.houses[3] = 2;
        let actions = state.gen_actions().collect::<Vec<_>>();
        assert_eq!(actions.len(), 13);
        // (len: 1; 0,)
        // (len: 1; 1,)
        // (len: 2; 2,0,)
        // (len: 2; 2,1,)
        // (len: 3; 2,3,0,)
        // (len: 3; 2,3,1,)
        // (len: 3; 2,3,4,)
        // (len: 3; 2,3,5,)
        // (len: 2; 2,4,)
        // (len: 2; 2,5,)
        // (len: 1; 3,)
        // (len: 1; 4,)
        // (len: 1; 5,)
        let mut state = GameState::new(0);
        state.houses[5] = 1;
        state.houses[10] = 1;
        let actions = state.gen_actions().collect::<Vec<_>>();
        assert_eq!(actions.len(), 1);
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
    fn test_end_game() {
        let mut state = GameState::new(4);
        assert_eq!(state.is_won(), None);
        state.finalize_game();
        assert_eq!(state.is_won(), Some(Tie));
        state.houses[13] = 50;
        assert_eq!(state.is_won(), Some(P2Win));
        state.houses[0] = 100;
        assert_eq!(state.is_won(), Some(P1Win));
        state.swap_board();
        assert_eq!(state.is_won(), Some(P2Win));
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

    #[test]
    fn test_player() {
        let mut state = GameState::new(4);
        let mut p1 = AIPlayer::new(state);
        let mut value_fun: HashMap<GameState, f64> = HashMap::new();
        let action = Action::singleton(4);
        state.evaluate_action(action);
        value_fun.insert(state, 10.0);

        assert_eq!(p1.take_action(&value_fun, 0.0), action);
        p1.td_update(&mut value_fun, 0.2, 0.3);
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

pub struct AIPlayer {
    curr_state: GameState,
    last_state: GameState,
}


pub trait Player {
    fn opponent_plays(&mut self, action: Action);
    fn current_state(&self) -> GameState;
    fn take_action(&mut self,
                   values: &HashMap<GameState, f64>,
                   epsilon: f64) -> Action;
    fn td_update(&self,
                 values: &mut HashMap<GameState, f64>,
                 learning_rate: f64,
                 discount_factor: f64);
}

const DEFAULT_STATE_VAL: f64 = 0.5f64;

impl AIPlayer {
    fn new(starting_state: GameState) -> AIPlayer {
        AIPlayer { curr_state: starting_state.clone(),
                 last_state: starting_state.clone() }
    }
}
impl Player for AIPlayer {
    fn opponent_plays(&mut self, action: Action) {
        self.last_state = self.curr_state;
        self.curr_state.swap_board();
        self.curr_state.evaluate_action(action);
        self.curr_state.swap_board();
    }

    fn take_action(&mut self,
                   values: &HashMap<GameState, f64>,
                   epsilon: f64) -> Action {
        let (action, _) = self.curr_state.pick_action(epsilon, values);
        debug!("Picked action {} at state \n{}", action, self.curr_state);
        self.curr_state.evaluate_action(action);
        debug!("Evaluated action {}, now at state\n{}", action, self.curr_state);
        action
    }

    fn td_update(&self,
                 values: &mut HashMap<GameState, f64>,
                 learning_rate: f64,
                 discount_factor: f64) {
        let q_next = *values.entry(self.curr_state).or_insert(DEFAULT_STATE_VAL);
        let q_last = values.entry(self.last_state).or_insert(DEFAULT_STATE_VAL);
        let q_tmp = *q_last; // just for printing
        *q_last += learning_rate * (discount_factor * q_next - q_tmp);
        debug!("Doing TD update from (self.last_state) q_last:\n{}\n\
               to (self.curr_state) q_next:\n{}",
               self.last_state, self.curr_state);
        debug!("q_last += learning_rate * (discount_factor * q_next - q_last)\n\
            {} += {} * ({} * {} - {})",
            *q_last, learning_rate, discount_factor, q_next, q_tmp);

    }

    fn current_state(&self) -> GameState {
        self.curr_state
    }
}

fn sarsa_loop(values: &mut HashMap<GameState, f64>,
              starting_state: GameState,
              epsilon: f64,
              learning_rate: f64,
              discount_factor: f64,
              episodes: usize) {
    let print_rate = 1000;
    let mut game_lengths = Vec::with_capacity(print_rate);
    dump_counter_stats(&game_lengths, true);
    
    for episode in 0..episodes {
        let mut current_player = AIPlayer::new(starting_state);
        let mut opposing_player = {
            let mut opp_starting_state = starting_state.clone();
            opp_starting_state.swap_board();
            AIPlayer::new(opp_starting_state)
        };
        info!(">>>>>>>>>>>>>>>>>");
        let mut counter = 0;
        loop {
            let players_turn = if counter % 2 == 0 { 1 } else { 2 };
            info!("Turn {}, player {}'s turn", counter, players_turn);

            let action = current_player.take_action(values, epsilon);
            opposing_player.opponent_plays(action);

            if current_player.curr_state.is_ended() {
                info!("Game ended at state:\n{}", current_player.curr_state);
                let (tie, curr_player_win) = {
                    let mut copy = current_player.curr_state.clone();
                    copy.finalize_game();
                    let diff = copy.houses[6] as i32 - copy.houses[13] as i32;
                    (diff == 0, diff > 0)
                };
                if curr_player_win {
                    values.insert(current_player.curr_state, 1.0);
                    values.insert(opposing_player.curr_state, 0.0);
                } else if !tie {
                    values.insert(current_player.curr_state, 0.0);
                    values.insert(opposing_player.curr_state, 1.0);
                }
                // The only reason this duplication has to happen here is because
                // we need to first set the terminal states to {1.0, 0.0}
                // otherwise we could move these four lines before the is_ended check
                // and remove the duplication
                debug!("TD Update for current player");
                current_player.td_update(values, learning_rate, discount_factor);
                debug!("TD Update for opposing player");
                opposing_player.td_update(values, learning_rate, discount_factor);

                counter += 1;
                game_lengths.push(counter);
                break;
            }
            debug!("TD Update for current player");
            current_player.td_update(values, learning_rate, discount_factor);
            debug!("TD Update for opposing player");
            opposing_player.td_update(values, learning_rate, discount_factor);
            counter += 1;
            std::mem::swap(&mut current_player, &mut opposing_player);
            info!(">>>>>>>>>>>>>>>>>");
        }
        if (episode+1) % print_rate == 0 {
            dump_counter_stats(&game_lengths, false);
            game_lengths.clear();
        }
    }
    dump_counter_stats(&game_lengths, false);
}

pub struct HumanPlayer{
    curr_state: GameState
}

impl HumanPlayer {
    fn new(starting_state: GameState) -> HumanPlayer {
        HumanPlayer { curr_state: starting_state.clone() }
    }
}

impl Player for HumanPlayer {
    fn opponent_plays(&mut self, action: Action) {
        self.curr_state.swap_board();
        self.curr_state.evaluate_action(action);
        self.curr_state.swap_board();
    }

    fn take_action(&mut self,
                   values: &HashMap<GameState, f64>,
                   _: f64) -> Action {
        println!("Computer went. State now (from your perspective):\n{}", self.curr_state);
        println!("\n----------------\n");
        println!("Now considering your options: ");
        for action in self.curr_state.gen_actions() {
            let mut state = self.curr_state;
            state.evaluate_action(action);
            println!("\n----------------\n{}:\n{}\nqval: {:?}\n", action, state, values.get(&state));
        }

        let choices: Vec<Action> = self.curr_state.gen_actions().collect();
        let index = loop {
            println!("Choose from these options:");
            for (i, choice) in choices.iter().enumerate() {
                println!("\t({}): {}", i, choice);
            }
            let mut input = String::new();
            use std::str::FromStr;
            use std::io::stdin;
            if let Err(_) = stdin().read_line(&mut input) {
                continue;
            }
            if let Ok(index) = u8::from_str(&input.trim()) {
                if (index as usize) < choices.len() {
                    break index
                }
            }
        };

        let action = choices[index as usize];
        debug!("Picked action {} at state \n{}", action, self.curr_state);
        self.curr_state.evaluate_action(action);
        debug!("Evaluated action {}, now at state\n{}", action, self.curr_state);
        println!("You played. State now:\n{}", self.curr_state);
        action
    }

    fn td_update(&self,
                 _: &mut HashMap<GameState, f64>,
                 _: f64,
                 _: f64) {}

    fn current_state(&self) -> GameState {
        self.curr_state
    }
}


fn play_loop(mut p1: Box<Player>, mut p2: Box<Player>,
             values: &mut HashMap<GameState, f64>,
             starting_state: GameState) {
    println!("Starting play loop:");
    println!("Starting state:\n{}", p1.current_state());
    loop {
        let action = p1.take_action(values, 0.0);
        p2.opponent_plays(action);
        if p1.current_state().is_ended() {
            break;
        }
        std::mem::swap(&mut p1, &mut p2);
    }
    println!("Game ended at state (from your perspective):\n{}", p1.current_state());
    match p1.current_state().is_won() {
        Some(P1Win) => println!("You won!"),
        Some(P2Win) => println!("You Lost!"),
        Some(Tie) => println!("Tied!?!"),
        _ => println!("Not over yet?"),
    }
}
    
fn main() {
    env_logger::init().unwrap();
    info!("Hello, mancala!");
    let args: Args = Docopt::new(USAGE)
                            .and_then(|d| d.decode())
                            .unwrap_or_else(|e| e.exit());

    let starting_state = GameState::new(1);
    println!("{}", starting_state);
    if args.cmd_train {
        let mut value_fun: HashMap<GameState, f64> = HashMap::with_capacity(1_000);
        sarsa_loop(&mut value_fun,
                   starting_state,
                   args.flag_epsilon,
                   args.flag_learning_rate,
                   args.flag_discount_rate,
                   args.flag_num_runs);

        println!("Number of entries in value function: {}", value_fun.len());

        let mut vals = value_fun.iter().collect::<Vec<_>>();
        vals.sort_by(|a, b| b.1.partial_cmp(a.1).unwrap());
        println!("Here's a few of the top values and states:");
        for pair in vals.iter().take(2) {
            println!("\n#########\n{}:\n", pair.1);
            println!("{}", pair.0);
        }
        vals.sort_by(|a, b| a.1.partial_cmp(b.1).unwrap());
        println!("Here's a few of the bottom values and states:");
        for pair in vals.iter().take(2) {
            println!("\n#########\nValue: {}:\n{}", pair.1, pair.0);
        }

        let encoded: Vec<u8> = encode(&value_fun, SizeLimit::Infinite).unwrap();
        let mut f: File = File::create(args.flag_train.unwrap_or("train.dat".to_string())).unwrap();
        f.write_all(&encoded).unwrap();
        drop(f);
    } else if args.cmd_play {
        let mut f: File = File::open(args.flag_train.unwrap_or("train.dat".to_string())).unwrap();
        let mut encoded = Vec::new();
        f.read_to_end(&mut encoded).unwrap();
        let mut value_fun: HashMap<GameState, f64> = decode(&encoded).unwrap();
        println!("Number of values in hash: {}", value_fun.len());
        println!();
        println!("Here are the first possible actions and their values: ");
        for action in starting_state.gen_actions() {
            let mut state = starting_state;
            state.evaluate_action(action);
            println!("\n----------------\n{}:\n{}\nqval: {:?}\n", action, state, value_fun.get(&state));
        }
        println!("\n----------------\n");

        let p1 = Box::new(HumanPlayer::new(starting_state));
        let p2 = Box::new({
            let mut opp_starting_state = starting_state.clone();
            opp_starting_state.swap_board();
            AIPlayer::new(opp_starting_state)
        });

        play_loop(p1, p2, &mut value_fun, starting_state);
    }

}
