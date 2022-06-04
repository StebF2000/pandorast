use rand::distributions::Uniform;
use serde::{Deserialize, Serialize};
use std::collections::{hash_map::Entry, HashMap};

use crate::iotwins_model::{agent::Agent, structures::Structure};

#[derive(Deserialize)]
struct RawArrival {
    index: usize,
    gate: String,
    mouth: u16,
    minutes_to_game: i32,
    agents: u8,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Arrival {
    pub gate: String, // Origin
    pub mouth: u16,   // Destination
    pub agents: u8,
}

impl Arrival {
    pub fn generate_agents(
        &self,
        target: &Structure,
        id_counting: usize,
        interest: Uniform<f32>,
    ) -> Vec<Agent> {
        (1..self.agents as usize)
            .map(|counter| Agent::new(id_counting + counter, target, self.mouth, interest))
            .collect()
    }

    pub fn gate_layer(&self) -> String {
        let id = self.gate.split('-').collect::<Vec<&str>>()[1].to_string();

        if vec![62, 63, 64, 73].contains(&id.parse::<i32>().unwrap()) {
            String::from("S1")
        } else {
            String::from("PB")
        }
    }

    pub fn mouth_layer(&self) -> String {
        if (100..153).contains(&self.mouth) {
            String::from("S1")
        } else if (200..211).contains(&self.mouth) {
            String::from("PB")
        } else if (212..222).contains(&self.mouth) || (240..250).contains(&self.mouth) {
            String::from("P0-5")
        } else if (223..239).contains(&self.mouth)
            || (300..325).contains(&self.mouth)
            || (345..358).contains(&self.mouth)
        {
            String::from("P1")
        } else if (313..357).contains(&self.mouth) {
            String::from("P2")
        } else if (414..456).contains(&self.mouth) {
            String::from("P3")
        } else {
            String::from("P4")
        }
    }
}

// Returns a hashmap with the list of agents to enter for each given time
pub fn load_arrivals() -> HashMap<i32, Vec<Arrival>> {
    let mut arrivals: HashMap<i32, Vec<Arrival>> = HashMap::new();

    let mut reader = csv::Reader::from_path("resources/tagging/arrivals.csv")
        .expect("[ERROR] Arrivals file not found");

    for result in reader.deserialize() {
        let record: RawArrival = result.expect("[ERROR] Incorrect gate format");

        match arrivals.entry(record.minutes_to_game) {
            Entry::Occupied(mut arrivals) => {
                arrivals.get_mut().push(Arrival {
                    gate: record.gate,
                    mouth: record.mouth,
                    agents: record.agents,
                });
            }
            Entry::Vacant(arrivals) => {
                arrivals.insert(vec![Arrival {
                    gate: record.gate,
                    mouth: record.mouth,
                    agents: record.agents,
                }]);
            }
        }
    }

    arrivals
}
