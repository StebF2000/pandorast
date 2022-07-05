#![crate_name = "pandorast"]
#![feature(drain_filter)]

mod config;
mod engine;

mod iotwins_model {
    pub mod agent;
    pub mod arrivals;
    pub mod config;
    pub mod routes;
    pub mod stadium;
    pub mod structures;
    pub mod world;
}

use indicatif::{ProgressBar, ProgressIterator, ProgressStyle};
use std::time::Instant;
// Microsoft memory allocator for performance
use mimalloc::MiMalloc;
use rand::distributions::Uniform;

use crate::iotwins_model::arrivals;

#[global_allocator]
static GLOBAL: MiMalloc = MiMalloc;

fn main() {
    println!("Welcome to Pandorast!");
    let start_time = Instant::now();

    // Multithreading configuration
    rayon::ThreadPoolBuilder::new()
        .num_threads(27)
        .build_global()
        .unwrap();

    // Create simulation
    let configuration =
        config::configuration::Parameters::load_configuration(String::from("IoTwins_config.toml"));

    let mut w = iotwins_model::world::create_world(configuration);
    w.bincode_save();
    w.save_structures();
    w.save_layer_paths();

    let interest = Uniform::from(0_f64..1_f64);

    // let mut w = iotwins_model::world::bincode_load(String::from("resources/IoTwins.bin"));
    // w.arrivals = arrivals::load_arrivals();

    // Progress bar
    let progress_bar = ProgressBar::new(40000);

    progress_bar.set_style(
        ProgressStyle::default_spinner()
            .template("{spinner}{elapsed_precise} {bar:40} {pos}/{len} {eta}")
            .progress_chars("#>#-"),
    );

    // End of progress bar

    let mut total_swapped = 0;

    // 0.6s per iteration (200 minutes)
    (0..40000).progress_with(progress_bar).for_each(|_| {
        // 250 minutes
        total_swapped += w.evolve(interest);
    });

    println!("[INFO] Simulation time: {:?}", start_time.elapsed());

    // Export pathing for each agent
    w.generate_save();

    // Agents correctly simulated
    let simulated_agents = w.agent_path.len();
    // Post-simulation information
    println!("[INFO] End of simulation");
    println!("[INFO] Total simulation: {:?}", start_time.elapsed());
    println!("[INFO] Total steps: {}", w.step);
    println!("[INFO] Total agents: {}", w.agent_count);
    println!("[INFO] Total agent with path: {}", simulated_agents);
    println!("[INFO] Total agents swapped: {total_swapped}");
    println!("[INFO] Agent lost: {}", w.agent_count - simulated_agents);
}
