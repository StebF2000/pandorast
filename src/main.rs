mod config;
mod model;
mod world;

// Microsoft memory allocator for performance
use mimalloc::MiMalloc;

#[global_allocator]
static GLOBAL: MiMalloc = MiMalloc;

fn main() {
    println!("Hello, world!");

    let configuration = config::config::Parameters::load_configuration(String::from("config.toml"));

    // println!("{:?}", configuration);

    let gates = match model::model::load_gates(configuration.venue_tags.gates_info) {
                Ok(file) => file,
                Err(error) => panic!("Problem opening the file: {:?}", error),
            };

    println!("{:?}", gates);
}
