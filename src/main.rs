mod config;
mod engine;

mod iotwins_model {
    pub mod agent;
    pub mod config;
    pub mod routes;
    pub mod stadium;
    pub mod structures;
    pub mod world;
}

// Microsoft memory allocator for performance
use mimalloc::MiMalloc;

use crate::engine::matrix::Matrix;

#[global_allocator]
static GLOBAL: MiMalloc = MiMalloc;

fn main() {
    println!("Welcome to Pandorast!");

    // Multithreading configuration
    rayon::ThreadPoolBuilder::new()
        .num_threads(16)
        .build_global()
        .unwrap();

    let configuration =
        config::configuration::Parameters::load_configuration(String::from("IoTwins_config.toml"));

    // let world = iotwins_model::world::create_world(configuration);

    // world.save_structures();
    // world.save_paths();

    let w = iotwins_model::world::load_world(
        "resources/627/map_jumps.json".to_string(),
        "resources/627/mouths_paths.json".to_string(),
        "resources/627/stairs_paths.json".to_string(),
        configuration,
    );
}
