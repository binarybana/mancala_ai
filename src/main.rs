use std::fs::File;
use std::io::{Read, Write};

extern crate clap;
extern crate postcard;
use crate::postcard::{from_bytes, to_allocvec};
use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(about = "Mancala AI using reinforcement learning.")]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Output/input training datafile.
    #[arg(short, long, value_name = "FILE")]
    train: Option<String>,

    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    Train {
        /// Number of complete games [default: 10].
        #[arg(short, long, value_name = "GAMES", default_value_t = 10)]
        num_runs: usize,
        /// Epsilon for non-greedy actions [default: 0.02].
        #[arg(short, long, value_name = "EPS", default_value_t = 0.02)]
        epsilon: f64,
        /// Discount rate [default: 1.0].
        #[arg(short, long, value_name = "DISC", default_value_t = 1.0)]
        discount_rate: f64,
        /// Learning rate [default: 0.05].
        #[arg(short, long, value_name = "DISC", default_value_t = 0.05)]
        learning_rate: f64,
    },
    Play {},
    /// Play with TUI interface showing move analysis
    PlayTUI {},
}

#[macro_use]
extern crate log;
extern crate env_logger;
extern crate rand;

use std::collections::HashMap;

mod learning;
mod mancala;
mod packed_actions;
mod player;
mod tui;

fn main() {
    env_logger::init();
    info!("Hello, mancala!");
    let args = Args::parse();

    let starting_state = mancala::GameState::new(4);
    println!("{}", starting_state);
    match &args.command {
        Some(Commands::Play {}) => {
            let mut f: File = File::open(args.train.unwrap_or("train.dat".to_string())).unwrap();
            let mut encoded = Vec::new();
            f.read_to_end(&mut encoded).unwrap();
            let mut value_fun: HashMap<mancala::GameState, f64> = from_bytes(&encoded).unwrap();
            println!("Number of values in hash: {}", value_fun.len());
            println!();
            println!("Here are the first possible actions and their values: ");
            for action in starting_state.gen_actions() {
                let mut state = starting_state;
                state.evaluate_action(action);
                println!(
                    "\n----------------\n{}:\n{}\nqval: {:?}\n",
                    action,
                    state,
                    value_fun.get(&state)
                );
            }
            println!("\n----------------\n");

            use player::{AIPlayer, HumanPlayer, Player};
            let p1 = Box::new(HumanPlayer::new(starting_state));
            let p2 = Box::new({
                let mut opp_starting_state = starting_state.clone();
                opp_starting_state.swap_board();
                AIPlayer::new(opp_starting_state)
            });

            player::play_loop(p1 as Box<dyn Player>, p2 as Box<dyn Player>, &mut value_fun);
        }
        Some(Commands::PlayTUI {}) => {
            let mut f: File = File::open(args.train.unwrap_or("train.dat".to_string())).unwrap();
            let mut encoded = Vec::new();
            f.read_to_end(&mut encoded).unwrap();
            let value_fun: HashMap<mancala::GameState, f64> = from_bytes(&encoded).unwrap();
            println!("Number of values in hash: {}", value_fun.len());
            println!("Starting TUI interface...");
            
            if let Err(err) = tui::run_tui(starting_state, &value_fun) {
                eprintln!("Error running TUI: {}", err);
            }
        }
        Some(Commands::Train {
            num_runs,
            epsilon,
            discount_rate,
            learning_rate,
        }) => {
            let mut value_fun: HashMap<mancala::GameState, f64> = HashMap::with_capacity(1_000);
            learning::sarsa_loop(
                &mut value_fun,
                starting_state,
                *epsilon,
                *learning_rate,
                *discount_rate,
                *num_runs,
            );

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

            let encoded: Vec<u8> = to_allocvec(&value_fun).unwrap();
            let mut f: File = File::create(args.train.unwrap_or("train.dat".to_string())).unwrap();
            f.write_all(&encoded).unwrap();
            drop(f);
        }
        None => {}
    }
}
