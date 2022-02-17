pub mod model {

    use std::collections::HashMap;

    use rand::distributions::Uniform;
    use serde::Deserialize;

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
        layout_s1: String,
    }

    impl Topology {
        pub fn layers(&self) -> HashMap<&str, &str> {
            HashMap::from([
                ("PB", self.layout_pb.as_str()),
                ("P05", self.layout_p05.as_str()),
                ("P1", self.layout_p1.as_str()),
                ("P15", self.layout_p15.as_str()),
                ("P2", self.layout_p2.as_str()),
                ("P3", self.layout_p3.as_str()),
                ("P35", self.layout_p35.as_str()),
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
}

pub mod world {

    use serde::Deserialize;
    use std::collections::HashMap;

    use crate::{
        config::configuration::Parameters,
        engine::matrix::Matrix,
    };

    #[derive(Debug, Deserialize)]
    struct Gate {
        gate: String,
        x: u32,
        y: u32,
    }

    /// HashMap of initial points (Gates). Key => usize position on matrix PB
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

    #[derive(Debug, Deserialize)]
    struct Mouth {
        mouth: String,
        layer: String,
        x: u32,
        y: u32,
    }

    /// HashMap of Hashmap. First by layer then by  mouth. Key => usize position in matrix
    pub fn load_mouths(path: String) -> HashMap<String, HashMap<String, Vec<usize>>> {
        let mut mouths: HashMap<String, HashMap<String, Vec<usize>>> = HashMap::new();

        let mut reader = csv::Reader::from_path(path).expect("[ERROR] Mouths file not found");

        for result in reader.deserialize() {
            let record: Mouth = result.expect("[ERROR] Incorrect mouth format");

            match mouths.get_mut(&record.layer) {
                // Layer key on HashMap exists
                Some(layer) => {
                    // Mouth is present in HashMap. Only position pushed

                    let mouths: Vec<String> =
                        record.mouth.split("-").map(|s| s.to_string()).collect();

                    for m in mouths {
                        // Some mouths feed people to the same grandstand

                        match layer.get_mut(&m) {
                            Some(mouth) => {
                                mouth.push((627 * record.x + record.y) as usize);
                            }
                            None => {
                                // Mouth not in HashMap. Gate vector position is created
                                layer.insert(m, vec![(627 * record.x + record.y) as usize]);
                            }
                        }
                    }
                }
                None => {
                    // Layer key on HashMap does not exist. All has to be created
                    mouths.insert(
                        record.layer,
                        HashMap::from([(record.mouth, vec![(627 * record.x + record.y) as usize])]),
                    );
                }
            }
        }

        mouths
    }

    #[derive(Debug)]
    struct Floor {
        name: String,
        blueprint: Matrix,
        agents: Matrix,
    }

    fn load_floor(floor: String, path: String, size: (u64, u64)) -> Floor {
        Floor {
            name: floor,
            blueprint: Matrix::load_layer(path),
            agents: Matrix::new(size),
        }
    }

    #[derive(Debug)]
    pub struct World {
        building: HashMap<String, Floor>,
        step: u64,
        total_agents: u64,
    }

    pub fn create_world(configuration: Parameters) -> World {
        let floors = configuration.topology.layers();
        let mut building: HashMap<String, Floor> = HashMap::new();

        for (floor, path) in floors {
            let mut layer = load_floor(
                floor.to_string(),
                path.to_string(),
                configuration.get_world_size(),
            );

            building.insert(floor.to_string(), layer);
        }

        World {
            building,
            step: 0,
            total_agents: configuration.total_agents(),
        }
    }
    
}
