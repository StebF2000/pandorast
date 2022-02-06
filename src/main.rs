mod config;
mod model;

fn main() {
    println!("Hello, world!");

    let configuration = config::config::Parameters::load_configuration(String::from("config.toml"));

    println!("{:?}", configuration);
}
