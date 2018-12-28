use super::player::{AIPlayer, Player};
use std::collections::HashMap;

fn dump_counter_stats(lens: &Vec<usize>, header_only: bool) {
    let buckets = vec![0, 5, 10, 15, 20, 25, 30, 45, 50, 60, 70, 80, 90, 100];
    if header_only {
        for buc in buckets.iter() {
            print!("[{:5}] ", buc);
        }
        println!("");
        return;
    }
    let mut counts = Vec::new();

    for i in 1..buckets.len() {
        let count = lens
            .iter()
            .filter(|val| **val < buckets[i] && **val >= buckets[i - 1])
            .count();
        counts.push(count);
    }
    for count in counts.iter() {
        print!("{:7} ", count);
    }
    println!("");
}

use mancala::GameState;

pub fn sarsa_loop(
    values: &mut HashMap<GameState, f64>,
    starting_state: GameState,
    epsilon: f64,
    learning_rate: f64,
    discount_factor: f64,
    episodes: usize,
) {
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
        if (episode + 1) % print_rate == 0 {
            dump_counter_stats(&game_lengths, false);
            game_lengths.clear();
        }
    }
    dump_counter_stats(&game_lengths, false);
}
