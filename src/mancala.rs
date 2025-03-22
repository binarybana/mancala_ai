use crate::packed_actions::{Action, ActionQueue, SubAction};
use rand::seq::SliceRandom;

extern crate serde;
use self::serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt::{self, Display, Formatter};

#[derive(Debug, PartialEq)]
pub enum Outcome {
    P1win,
    P2win,
    Tie,
}

#[derive(Debug, Eq, PartialEq, Hash, Copy, Clone, Serialize, Deserialize)]
pub struct GameState {
    pub houses: [u8; 14],
}

impl GameState {
    /// Create a new board initialized with each house having `starting_seeds` number of seeds.
    pub fn new(starting_seeds: u8) -> GameState {
        let mut state = GameState {
            houses: [starting_seeds; 14],
        };
        state.houses[6] = 0;
        state.houses[13] = 0;
        state
    }

    /// Is the game completely over where one player has emptied their side of the board?
    pub fn is_ended(&self) -> bool {
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
    pub fn is_won(&self) -> Option<Outcome> {
        let p1_tot: u8 = self.houses[..6].iter().sum();
        let p2_tot: u8 = self.houses[7..13].iter().sum();
        if p1_tot != 0 && p2_tot != 0 {
            return None;
        }
        let p1_tot = p1_tot + self.houses[6];
        let p2_tot = p2_tot + self.houses[13];
        use self::Outcome::*;
        if p1_tot > p2_tot {
            Some(P1win)
        } else if p2_tot > p1_tot {
            Some(P2win)
        } else {
            Some(Tie)
        }
    }

    /// Move remaining seeds to the appropriate player's store after a game ends
    pub fn finalize_game(&mut self) {
        // Assert that the game is actually over (at least one side is empty)
        let p1_empty = self.houses[..6].iter().sum::<u8>() == 0;
        let p2_empty = self.houses[7..13].iter().sum::<u8>() == 0;
        assert!(p1_empty || p2_empty, "Cannot finalize a game that is not over");
        
        // If player 1's side is empty, move player 2's remaining seeds to their store
        if p1_empty {
            for i in 7..13 {
                self.houses[13] += self.houses[i];
                self.houses[i] = 0;
            }
        } 
        // If player 2's side is empty, move player 1's remaining seeds to their store
        else if p2_empty {
            for i in 0..6 {
                self.houses[6] += self.houses[i];
                self.houses[i] = 0;
            }
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
    pub fn evaluate_action(&mut self, mut action_list: Action) {
        // TODO: make this a proper iterator
        // for each action in action_list
        loop {
            let subaction = action_list.pop_front();
            self.evaluate_subaction(subaction);
            if action_list.is_empty() {
                break;
            }
        }
    }

    /// Mutate the current game state when playing out a single subaction
    fn evaluate_subaction(&mut self, subaction: SubAction) {
        let action = subaction as usize;
        assert!(action != 6 && action != 13);
        let seeds = self.houses[action] as usize;
        // Pickup seeds from starting house
        self.houses[action] = 0;
        let end_house = action + seeds % 14;
        // Deposit seeds in each house around the board
        // Offset is to handle skipping of the opponents
        // scoring house as we go around the loop
        let mut offset = 0;
        for i in action + 1..end_house + 1 {
            if i > 0 && i % 13 == 0 {
                self.houses[0] += 1;
                offset += 1;
            } else {
                self.houses[(i + offset) % 14] += 1;
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

    pub fn gen_actions(&self) -> ActionIter {
        ActionIter {
            action: Action::new(),
            base_state: &self,
            state_stack: Vec::new(),
        }
    }

    pub fn pick_action(self, epsilon: f64, values: &ValueFunction) -> (Action, f64) {
        let choices: Vec<(Action, f64)> = self
            .gen_actions()
            .map(|action| (action, self.evaluate_to_new_state(action)))
            .map(|(action, possible_state)| {
                (action, *values.get(&possible_state).unwrap_or(&0.5f64))
            })
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
            let mut rng = rand::thread_rng();
            best = choices.choose(&mut rng).unwrap();
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
    pub fn swap_board(&mut self) {
        let n = self.houses.len();
        for i in 0..n / 2 {
            let temp = self.houses[i];
            self.houses[i] = self.houses[n / 2 + i];
            self.houses[n / 2 + i] = temp;
        }
    }

    fn find_next_subaction(&self, search_start: SubAction) -> Option<SubAction> {
        for index in search_start..6 {
            if self.houses[index as usize] > 0 {
                return Some(index);
            }
        }
        None
    }
}

pub struct ActionIter<'a> {
    action: Action,
    base_state: &'a GameState,
    state_stack: Vec<GameState>,
}

impl<'a> ActionIter<'a> {
    fn get_current_state(&self) -> GameState {
        if self.state_stack.is_empty() {
            self.base_state.clone()
        } else {
            self.state_stack[self.state_stack.len() - 1]
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
        if self.action.is_empty() {
            // need to initialize search at terminal state
            if let Some(sub) = self.base_state.find_next_subaction(0) {
                trace!(
                    "self.action was empty, initializing search and found next_subaction: {:?}",
                    sub
                );
                trace!("Now calling `next_terminal_state`");
                self.next_terminal_state(sub);
                return Some(self.action);
            }
            return None;
        }
        loop {
            let curr_state = self.get_current_state();
            trace!(
                "self.action was not empty: {}, finding next subaction",
                self.action
            );
            let prev_subaction = self.action.pop_back();
            trace!("base_state: \n{}", self.base_state);
            trace!("curr_state: \n{}", curr_state);
            trace!("popped subaction {} off self.action", prev_subaction);
            if let Some(sub) = curr_state.find_next_subaction(prev_subaction + 1) {
                self.next_terminal_state(sub);
                return Some(self.action);
            } else {
                trace!("Couldn't find any subactions at state\n{}", curr_state);
                if self.state_stack.is_empty() || curr_state.is_ended() {
                    // all done
                    trace!("All done");
                    return None;
                } else {
                    // try the next parent subaction
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
        write!(
            f,
            "+-------------------------------+\n\
             |   |"
        )?;
        
        // player 2 cells
        for house in self.houses[7..13].iter().rev() {
            write!(f, "{:2} |", house)?;
        }
        
        // end zones
        write!(
            f,
            "   |\n|{:2} |                       |{:2} |\n\
             |   |",
            self.houses[13], self.houses[6]
        )?;
        
        // player 1 cells
        for house in &self.houses[0..6] {
            write!(f, "{:2} |", house)?;
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
        let expected: [u8; 14] = [0, 5, 5, 5, 0, 4, 5, 4, 0, 4, 4, 4, 4, 0];
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
        assert_eq!(state.houses[13], 4 * 6);
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
        let mut p1 = crate::player::AIPlayer::new(state);
        let mut value_fun: HashMap<GameState, f64> = HashMap::new();
        let action = Action::singleton(4);
        state.evaluate_action(action);
        value_fun.insert(state, 10.0);

        assert_eq!(p1.take_action(&value_fun, 0.0), action);
        p1.td_update(&mut value_fun, 0.2, 0.3);
    }
}
