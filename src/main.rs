mod config;
mod engine;
mod model;

// Microsoft memory allocator for performance
use mimalloc::MiMalloc;

#[global_allocator]
static GLOBAL: MiMalloc = MiMalloc;

fn main() {
    println!("Welcome to Pandorast!");

    let configuration = config::config::Parameters::load_configuration(String::from("config.toml"));

    let age = rand::distributions::Uniform::new(
        configuration.get_agent_info().min_age,
        configuration.get_agent_info().max_age,
    );

    let world = model::world::create_world(configuration);
}
