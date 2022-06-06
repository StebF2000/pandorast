#![crate_name = "pandorast"]

mod config;
mod engine;

mod iotwins_model {
    pub mod agent;
    pub mod arrivals;
    pub mod config;
    pub mod routes;
    pub mod snapshot;
    pub mod stadium;
    pub mod structures;
    pub mod world;
}

// Microsoft memory allocator for performance
use mimalloc::MiMalloc;
use rand::distributions::Uniform;

use crate::{
    engine::matrix::Position,
    iotwins_model::{routes::Route, structures::Structure},
};

#[global_allocator]
static GLOBAL: MiMalloc = MiMalloc;

fn main() {
    println!("Welcome to Pandorast!");

    // Multithreading configuration
    rayon::ThreadPoolBuilder::new()
        .num_threads(20)
        .build_global()
        .unwrap();

    let configuration =
        config::configuration::Parameters::load_configuration(String::from("IoTwins_config.toml"));

    let interest = Uniform::from(0_f32..1_f32);

    // let mut w = iotwins_model::world::create_world(configuration);
    // w.bincode_save();

    // let mut w = iotwins_model::world::load_world(
    //     "resources/stairs.json".to_string(),
    //     "resources/stairs_paths".to_string(),
    //     "resources/mouths_paths".to_string(),
    //     &configuration,
    // );

    let mut w = iotwins_model::world::bincode_load(String::from("resources/IoTwins.bin"));

    // 0.3s per iteration -> total of 150 minutes
    while w.step < 44000 {
        w.evolve(interest);
    }
}
