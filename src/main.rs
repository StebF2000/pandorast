mod config;
mod engine;
mod iotwins;

// Microsoft memory allocator for performance
use mimalloc::MiMalloc;

#[global_allocator]
static GLOBAL: MiMalloc = MiMalloc;

fn main() {
    println!("Welcome to Pandorast!");

    let configuration =
        config::configuration::Parameters::load_configuration(String::from("IoTwins_config.toml"));

    let world = iotwins::world::create_world(configuration);

    // iotwins::world::MapJump::save_locations(&world.stairs, String::from("locations"));

    let paths = world.default_paths();

    println!("{:?}", paths);

}
