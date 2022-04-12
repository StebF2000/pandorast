pub mod model {

    use serde::Deserialize;
    use std::collections::HashMap;

    #[derive(Debug, Deserialize, Clone, Copy)]
    pub struct AgentStats {
        min_vision: u8,
        max_vision: u8,
        min_velocity: u8,
        max_velocity: u8,
        pub min_age: u8,
        pub max_age: u8,
        porv_tourist: f32,
        min_wall_distance: u8,
        max_wall_distance: u8,
        min_agent_distance: u8,
        max_agent_distance: u8,
        max_distance_b_agents: u8,
        prov_follow: u8,
        prov_museum: u8,
    }

    #[derive(Debug, Deserialize, Clone, Copy)]
    pub struct Coeffs {
        calpha: f32,
        cbeta: f32,
        cdelta: f32,
        csigma: f32,
        ualpha: f32,
        ubeta: f32,
        udelta: f32,
        usigma: f32,
    }

    #[derive(Debug, Deserialize)]
    pub struct Topology {
        layout_pb: String,
        layout_p05: String,
        layout_p1: String,
        layout_p15: String,
        layout_p2: String,
        layout_p3: String,
        layout_p35: String,
        layout_p4: String,
        layout_p5: String,
    }

    impl Topology {
        pub fn layers(&self) -> HashMap<&str, &str> {
            HashMap::from([
                ("PB", self.layout_pb.as_str()),
                ("P0-5", self.layout_p05.as_str()),
                ("P1", self.layout_p1.as_str()),
                ("P1-5", self.layout_p15.as_str()),
                ("P2", self.layout_p2.as_str()),
                ("P3", self.layout_p3.as_str()),
                ("P3-5", self.layout_p35.as_str()),
                ("P4", self.layout_p4.as_str()),
                ("P5", self.layout_p5.as_str()),
            ])
        }
    }

    #[derive(Debug, Deserialize)]
    pub struct Venue {
        pub gates_info: String,
        pub mouths_info: String,
        pub arrivals_info_csv: String,
    }

    #[derive(Debug, Deserialize, Clone, Copy)]
    pub struct Match {
        match_start: f32,
        seconds_per_step: f32,
        distribute_agents_along_minutes: bool,
    }

    #[derive(Debug, Deserialize)]
    struct ArrivalData {
        gate: String,
        mouth: String,
        minutes_to_game: f32,
        agents: u8,
    }

    #[derive(Debug)]
    pub struct Arrival {
        init: String,
        destination: String,
        num_agents: u8,
    }

    //HashMap of arrivals by time. Key => vec of arrivals
    pub fn load_arrivals(path: String) -> HashMap<i32, Vec<Arrival>> {
        let mut arrivals: HashMap<i32, Vec<Arrival>> = HashMap::new();

        let mut reader = csv::Reader::from_path(path).expect("[ERROR] Arrivals file not found");

        for arrival in reader.deserialize() {
            let data: ArrivalData = arrival.expect("[ERROR] Incorrect arrival format");

            match arrivals.get_mut(&(data.minutes_to_game as i32)) {
                Some(time) => time.push(Arrival {
                    // Pushes new arrival
                    init: data.gate,
                    destination: data.mouth,
                    num_agents: data.agents,
                }),
                None => {
                    // Generates key and vector with first arrival
                    arrivals.insert(
                        data.minutes_to_game as i32,
                        vec![Arrival {
                            init: data.gate,
                            destination: data.mouth,
                            num_agents: data.agents,
                        }],
                    );
                }
            }
        }

        arrivals
    }

    #[derive(Debug)]
    pub struct Agent {
        pub id: u64,
        age: u32,
        path: Vec<usize>,
        pub init: String,
        pub destination: String,
        pub position: usize,
        pub layer: String,
        steps: usize,
    }

    impl Agent {
        // pub fn load_agents(
        //     arrival_data: &[Arrival],
        //     world: &mut World,
        //     index: &mut u32,
        //     rng: &mut ThreadRng,
        // ) {
        //     // Update index for agent id
        //     let mut idx = *index;

        //     for arrival in arrival_data {
        //         for _ in 0..arrival.num_agents {
        //             let mut agent = Agent {
        //                 id: idx,
        //                 age: 25, // Not implemented as not used yet. Default age set
        //                 path: vec![0, 1, 2],
        //                 init: arrival.init.to_string(), // Gate reference to place the agent
        //                 destination: arrival.destination,
        //                 position: 0, // None alternative
        //                 layer: String::from("PB"),
        //                 steps: 0,
        //             };

        //             idx += 1;

        //             // Placing agent on gate
        //             world.insert_agent(&mut agent, rng);
        //         }
        //     }
        //     // Updated index for next agents
        //     *index = idx;
        // }

        pub fn action_movement(&mut self) -> usize {
            self.steps += 1;

            self.path[self.steps]
        }
    }
}

pub mod world {

    use indicatif::{ParallelProgressIterator, ProgressBar, ProgressStyle};
    use rayon::iter::{IntoParallelRefIterator, ParallelIterator};
    use serde::{Deserialize, Serialize};
    use std::{
        collections::{BinaryHeap, HashMap, HashSet},
        fs::File,
        iter::zip,
    };

    use crate::{
        config::configuration::Parameters,
        engine::matrix::Matrix,
        engine::{self, matrix::Position, routes::ConcurrentHashMap},
        iotwins::{
            self,
            model::{self, Arrival},
            world,
        },
        iotwins_model::{structures::{self, tagging::Jump}, stadium::environment::Floor},
    };

    #[derive(Debug, Deserialize)]
    struct Mouth {
        mouth: String,
        layer: String,
        x: u32,
        y: u32,
    }

    /// HashMap of Hashmap. First by layer then by  mouth. Key => usize position in matrix
    pub fn load_mouths(path: String) -> HashMap<String, HashMap<i16, Vec<usize>>> {
        let mut mouths: HashMap<String, HashMap<i16, Vec<usize>>> = HashMap::new();

        let mut reader = csv::Reader::from_path(path).expect("[ERROR] Mouths file not found");

        for result in reader.deserialize() {
            let record: Mouth = result.expect("[ERROR] Incorrect mouth format");

            let multiple: Vec<i16> = record
                .mouth
                .split('-')
                .map(|s| s.parse().unwrap())
                .collect();

            match mouths.get_mut(&record.layer) {
                Some(layer) => {
                    // Mouth is present in HashMap. Only position pushed
                    for section in multiple {
                        // Some mouths feed people to the same grandstand
                        match layer.get_mut(&section) {
                            Some(m) => {
                                m.push((627 * record.x + record.y) as usize);
                            }
                            None => {
                                // Mouth not in HashMap. Gate vector position is created
                                layer.insert(section, vec![(627 * record.x + record.y) as usize]);
                            }
                        }
                    }
                }
                None => {
                    let layer = record.layer.clone();
                    // Layer key on HashMap does not exist. All has to be created
                    mouths.insert(record.layer, HashMap::new());

                    if let Some(layer) = mouths.get_mut(&layer) {
                        // Mouth is present in HashMap. Only position pushed
                        for section in multiple {
                            // Some mouths feed people to the same grandstand
                            if let Some(m) = layer.get_mut(&section) {
                                m.push((627 * record.x + record.y) as usize);
                            } else {
                                // Mouth not in HashMap. Gate vector position is created
                                layer.insert(section, vec![(627 * record.x + record.y) as usize]);
                            }
                        }
                    }
                }
            }
        }

        mouths
    }

    #[derive(Debug, Deserialize)]
    struct Gate {
        gate: String,
        x: u32,
        y: u32,
    }

    // HashMap of initial points (Gates). Key => usize position on matrix PB
    pub fn load_gates(path: String) -> HashMap<String, Vec<usize>> {
        let mut gates: HashMap<String, Vec<usize>> = HashMap::new();

        let mut reader = csv::Reader::from_path(path).expect("[ERROR] Gates file not found");

        for result in reader.deserialize() {
            let record: Gate = result.expect("[ERROR] Incorrect gate format");

            match gates.get_mut(&record.gate) {
                Some(gate) => {
                    gate.push((627 * record.x + record.y) as usize);
                }
                None => {
                    gates.insert(record.gate, vec![(627 * record.x + record.y) as usize]);
                }
            }
        }

        gates
    }
}
