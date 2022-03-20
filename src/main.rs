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

    let jumps = iotwins::world::MapJump::find_location(world);

    serde_json::to_writer(&File::create("stairs.json").expect("ERROR"), &jumps).expect("[ERROR]");

}
