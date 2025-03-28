use crate::mancala::GameState;
use crate::packed_actions::Action;
use std::collections::HashMap;

pub trait Player {
    fn opponent_plays(&mut self, action: Action);
    fn current_state(&self) -> GameState;
    fn take_action(&mut self, values: &HashMap<GameState, f64>, epsilon: f64) -> Action;
    fn td_update(
        &self,
        values: &mut HashMap<GameState, f64>,
        learning_rate: f64,
        discount_factor: f64,
    );
}

const DEFAULT_STATE_VAL: f64 = 0.5f64;

pub struct AIPlayer {
    pub curr_state: GameState,
    pub last_state: GameState,
}

impl AIPlayer {
    pub fn new(starting_state: GameState) -> AIPlayer {
        AIPlayer {
            curr_state: starting_state.clone(),
            last_state: starting_state.clone(),
        }
    }
}

impl Player for AIPlayer {
    fn opponent_plays(&mut self, action: Action) {
        self.last_state = self.curr_state;
        self.curr_state.swap_board();
        self.curr_state.evaluate_action(action);
        self.curr_state.swap_board();
    }

    fn take_action(&mut self, values: &HashMap<GameState, f64>, epsilon: f64) -> Action {
        let (action, _) = self.curr_state.pick_action(epsilon, values);
        debug!("Picked action {} at state \n{}", action, self.curr_state);
        self.curr_state.evaluate_action(action);
        debug!(
            "Evaluated action {}, now at state\n{}",
            action, self.curr_state
        );
        action
    }

    fn td_update(
        &self,
        values: &mut HashMap<GameState, f64>,
        learning_rate: f64,
        discount_factor: f64,
    ) {
        let q_next = *values.entry(self.curr_state).or_insert(DEFAULT_STATE_VAL);
        let q_last = values.entry(self.last_state).or_insert(DEFAULT_STATE_VAL);
        let q_tmp = *q_last; // just for printing
        *q_last += learning_rate * (discount_factor * q_next - q_tmp);
        debug!(
            "Doing TD update from (self.last_state) q_last:\n{}\n\
             to (self.curr_state) q_next:\n{}",
            self.last_state, self.curr_state
        );
        debug!(
            "q_last += learning_rate * (discount_factor * q_next - q_last)\n\
             {} += {} * ({} * {} - {})",
            *q_last, learning_rate, discount_factor, q_next, q_tmp
        );
    }

    fn current_state(&self) -> GameState {
        self.curr_state
    }
}

pub struct HumanPlayer {
    curr_state: GameState,
}

impl HumanPlayer {
    pub fn new(starting_state: GameState) -> HumanPlayer {
        HumanPlayer {
            curr_state: starting_state.clone(),
        }
    }
}

impl Player for HumanPlayer {
    fn opponent_plays(&mut self, action: Action) {
        self.curr_state.swap_board();
        self.curr_state.evaluate_action(action);
        self.curr_state.swap_board();
    }

    fn take_action(&mut self, values: &HashMap<GameState, f64>, _: f64) -> Action {
        println!(
            "Computer went. State now (from your perspective):\n{}",
            self.curr_state
        );
        println!("\n----------------\n");
        println!("Now considering your options: ");
        for action in self.curr_state.gen_actions() {
            let mut state = self.curr_state;
            state.evaluate_action(action);
            println!(
                "\n----------------\n{}:\n{}\nqval: {:?}\n",
                action,
                state,
                values.get(&state)
            );
        }

        let choices: Vec<Action> = self.curr_state.gen_actions().collect();
        let index = loop {
            println!("Choose from these options:");
            for (i, choice) in choices.iter().enumerate() {
                println!("\t({}): {}", i, choice);
            }
            let mut input = String::new();
            use std::io::stdin;
            use std::str::FromStr;
            if let Err(_) = stdin().read_line(&mut input) {
                continue;
            }
            if let Ok(index) = u8::from_str(&input.trim()) {
                if (index as usize) < choices.len() {
                    break index;
                }
            }
        };

        let action = choices[index as usize];
        debug!("Picked action {} at state \n{}", action, self.curr_state);
        self.curr_state.evaluate_action(action);
        debug!(
            "Evaluated action {}, now at state\n{}",
            action, self.curr_state
        );
        println!("You played. State now:\n{}", self.curr_state);
        action
    }

    fn td_update(&self, _: &mut HashMap<GameState, f64>, _: f64, _: f64) {}

    fn current_state(&self) -> GameState {
        self.curr_state
    }
}

pub fn play_loop(
    mut p1: Box<dyn Player>,
    mut p2: Box<dyn Player>,
    values: &mut HashMap<GameState, f64>,
) {
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
    // Get a mutable copy of the current state to properly finalize it
    let mut final_state = p1.current_state().clone();
    // Finalize the game to move stones to the correct stores
    final_state.finalize_game();
    
    println!(
        "Game ended at state (from your perspective):\n{}",
        final_state
    );
    
    use crate::mancala::Outcome::*;
    match final_state.is_won() {
        Some(P1win) => println!("You won! Final score: {}-{}", 
                               final_state.houses[6], final_state.houses[13]),
        Some(P2win) => println!("You Lost! Final score: {}-{}", 
                              final_state.houses[6], final_state.houses[13]),
        Some(Tie) => println!("It's a tie! Final score: {}-{}", 
                            final_state.houses[6], final_state.houses[13]),
        _ => println!("Not over yet?"),
    }
}
