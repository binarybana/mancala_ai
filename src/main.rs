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

use std::collections::HashMap;

mod packed_actions;
mod mancala;
mod player;
mod learning;

fn main() {
    env_logger::init().unwrap();
    info!("Hello, mancala!");
    let args: Args = Docopt::new(USAGE)
                            .and_then(|d| d.decode())
                            .unwrap_or_else(|e| e.exit());

    let starting_state = mancala::GameState::new(1);
    println!("{}", starting_state);
    if args.cmd_train {
        let mut value_fun: HashMap<mancala::GameState, f64> = HashMap::with_capacity(1_000);
        learning::sarsa_loop(&mut value_fun,
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
        let mut value_fun: HashMap<mancala::GameState, f64> = decode(&encoded).unwrap();
        println!("Number of values in hash: {}", value_fun.len());
        println!();
        println!("Here are the first possible actions and their values: ");
        for action in starting_state.gen_actions() {
            let mut state = starting_state;
            state.evaluate_action(action);
            println!("\n----------------\n{}:\n{}\nqval: {:?}\n", action, state, value_fun.get(&state));
        }
        println!("\n----------------\n");

        use player::{HumanPlayer, AIPlayer, Player};
        let p1 = Box::new(HumanPlayer::new(starting_state));
        let p2 = Box::new({
            let mut opp_starting_state = starting_state.clone();
            opp_starting_state.swap_board();
            AIPlayer::new(opp_starting_state)
        });

        player::play_loop(p1 as Box<Player>, p2 as Box<Player>, &mut value_fun, starting_state);
    }
}
