mod config;
mod engine;

mod iotwins_model {
    pub mod config;
    pub mod stadium;
    pub mod structures;
    pub mod world;
}

// Microsoft memory allocator for performance
use mimalloc::MiMalloc;

use crate::iotwins_model::{structures, world};

#[global_allocator]
static GLOBAL: MiMalloc = MiMalloc;

fn main() {
    println!("Welcome to Pandorast!");

    let configuration =
        config::configuration::Parameters::load_configuration(String::from("IoTwins_config.toml"));

    let world = world::simulation::create_world(configuration);

    structures::tagging::Jump::save_locations(
        &world.stairs,
        String::from("resources/627/map_jumps/locations"),
    );

    let paths = world.default_paths();

    println!("{:?}", paths);
}
