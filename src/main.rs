mod config;
mod engine;

mod iotwins_model {
    pub mod config;
    pub mod stadium;
    pub mod structures;
    pub mod world;
    pub mod agent;
}

// Microsoft memory allocator for performance
use mimalloc::MiMalloc;

#[global_allocator]
static GLOBAL: MiMalloc = MiMalloc;

fn main() {
    println!("Welcome to Pandorast!");

    let configuration =
        config::configuration::Parameters::load_configuration(String::from("IoTwins_config.toml"));

    let world = iotwins_model::world::create_world(configuration);


    let paths = world.default_paths();

    println!("{:?}", paths);
}
