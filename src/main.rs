mod config;
mod engine;
mod iotwins;

use std::fs::File;

// Microsoft memory allocator for performance
use mimalloc::MiMalloc;

#[global_allocator]
static GLOBAL: MiMalloc = MiMalloc;

fn main() {
    println!("Welcome to Pandorast!");

    let mut rng = rand::thread_rng();

    let configuration =
        config::configuration::Parameters::load_configuration(String::from("config.toml"));

    let world = iotwins::world::create_world(configuration);


}
