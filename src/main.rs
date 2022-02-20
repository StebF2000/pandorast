mod config;
mod engine;
mod iotwins_model;
mod iotwins_world;

use rand::prelude::*;

// Microsoft memory allocator for performance
use mimalloc::MiMalloc;

#[global_allocator]
static GLOBAL: MiMalloc = MiMalloc;

fn main() {
    println!("Welcome to Pandorast!");

    let mut rng = rand::thread_rng();

    let configuration =
        config::configuration::Parameters::load_configuration(String::from("config.toml"));

    let age = rand::distributions::Uniform::new(
        configuration.get_agent_info().min_age,
        configuration.get_agent_info().max_age,
    );

    let mut world = iotwins_world::world::World::create_world(&configuration);

    let arrival_data =
        iotwins_model::model::Arrival::load_arrivals(&configuration.venue_tags.arrivals_info_csv);

    let test = arrival_data.get(&-20).expect("msg");

    let mut ag_index = 0_u32;

    iotwins_model::model::Agent::load_agents(test, &mut world, &mut ag_index, &mut rng);

    println!("{ag_index}");
}
