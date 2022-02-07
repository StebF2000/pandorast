mod config;
mod model;
mod world;

use mimalloc::MiMalloc;

#[global_allocator]
static GLOBAL: MiMalloc = MiMalloc;

fn main() {
    println!("Hello, world!");

    let configuration = config::config::Parameters::load_configuration(String::from("config.toml"));

    println!("{:?}", configuration);
}
