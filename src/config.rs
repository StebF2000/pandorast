use serde::{Deserialize};
mod agent;

#[derive(Debug, Deserialize)]
struct Output {
    results_dir: String,
    results_file: String,
    logs_dir: String,
    logs_console: bool,
    instrumentation: bool,
}

#[derive(Debug, Deserialize)]
struct Simulation {
    steps: u64,
    resolution: u64,
    height: u64,
    width: u64,
    agents: u64,
    counters: u64,
    match_start: f32,
    seconds_step: f32,
    distribute_agents: bool,
}

#[derive(Debug, Deserialize)]
pub struct Config {
    outputs: Output,
    simulation: Simulation,
    agents: agent::Agent,
}

pub fn load_configuration(path: String) -> Config {

    // Open config file
    let data = match std::fs::read_to_string(path) {
        Ok(file) => file,
        Err(error) => panic!("Problem opening the file: {:?}", error),
    };

    // Deserialize config file into config struct
    let value: Config = match toml::from_str(&data) {
        Ok(file) => file,
        Err(error) => panic!("Problem opening the file: {:?}", error),
    };

    return value

}