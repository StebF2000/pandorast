pub mod world {

    use rand::prelude::{SliceRandom, ThreadRng};
    use serde::Deserialize;
    use std::collections::HashMap;

    use crate::{
        config::configuration::Parameters,
        engine::matrix::{Buffer, Grid},
        iotwins_model::model,
    };

    #[derive(Debug, Deserialize)]
    struct Mouth {
        mouth: String,
        layer: String,
        x: u32,
        y: u32,
    }

    impl Mouth {
        /// HashMap of Hashmap. First by layer then by  mouth. Key => usize position in matrix
        pub fn load_mouths(path: &str) -> HashMap<String, HashMap<String, Vec<usize>>> {
            let mut mouths: HashMap<String, HashMap<String, Vec<usize>>> = HashMap::new();

            let mut reader = csv::Reader::from_path(path).expect("[ERROR] Mouths file not found");

            for result in reader.deserialize() {
                let record: Mouth = result.expect("[ERROR] Incorrect mouth format");

                match mouths.get_mut(&record.layer) {
                    // Layer key on HashMap exists
                    Some(layer) => {
                        // Mouth is present in HashMap. Only position pushed

                        let mouths: Vec<String> =
                            record.mouth.split('-').map(|s| s.to_string()).collect();

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
                            HashMap::from([(
                                record.mouth,
                                vec![(627 * record.x + record.y) as usize],
                            )]),
                        );
                    }
                }
            }

            mouths
        }
    }

    #[derive(Debug, Deserialize)]
    struct Gate {
        gate: String,
        x: u32,
        y: u32,
    }

    impl Gate {
        /// HashMap of initial points (Gates). Key => usize position on matrix PB
        pub fn load_gates(path: &str) -> HashMap<String, Vec<usize>> {
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

    #[derive(Debug)]
    pub struct Buffer {
        queue: HashMap<Agent, u8>,
        wait: u8,
    }

    impl Buffer {
        // Creates an empty buffer for a location with fixed size
        pub fn new(size: usize, wait: u8) -> Buffer {
            Buffer {
                queue: HashMap::with_capacity(size),
                wait,
            }
        }

        pub fn move_layer(&mut self) {
            
        }
    }

    #[derive(Debug)]
    pub struct Floor {
        blueprint: Grid,
        agents: Grid,
        buffers: HashMap<usize, Buffer>,
    }

    impl Floor {
        pub fn load_floor(
            path: String,
            size: (usize, usize),
            buffer_size: usize,
            wait: u8,
        ) -> Floor {
            Floor {
                blueprint: Grid::load_layer(&path),
                agents: Grid::new(size),
                buffers: Floor::create_buffers(Grid::load_layer(&path), buffer_size, wait),
            }
        }

        fn create_buffers(blueprint: Grid, size: usize, wait: u8) -> HashMap<usize, Buffer> {
            let mut buffers: HashMap<usize, Buffer> = HashMap::new();

            blueprint
                .data
                .iter()
                .position(|x| x == &104_u32 || x == &190_u32)
                .into_iter()
                .for_each(|stair| {
                    buffers.insert(stair, Buffer::new(size, wait));
                });

            buffers
        }

        pub fn place_agent(&mut self, agent: &mut model::Agent, position: usize) {
            // Place agent ID on grid
            self.agents.matrix_movement(agent, position);
            // Update agent position
            agent.position = position;
        }

        pub fn save_state(&self) {
            self.agents.write_data()
        }
    }

    #[derive(Debug)]
    pub struct World {
        pub building: HashMap<String, Floor>, // Collection of floors by string
        pub step: u64,
        pub total_agents: u64,
        gates: HashMap<String, Vec<usize>>,
        mouths: HashMap<String, HashMap<String, Vec<usize>>>,
    }

    impl World {
        // Generate a unique HashMap with the whole simulation with index for checkpointing and agents
        pub fn create_world(configuration: &Parameters) -> World {
            let floors = configuration.topology.layers();

            // HashMap with all layers
            let mut building: HashMap<String, Floor> = HashMap::new();

            println!("[INFO] Loading venue...");
            for (floor, path) in floors {
                let mut layer =
                    Floor::load_floor(path.to_string(), configuration.get_world_size(), 10, 0);

                building.insert(floor.to_string(), layer);
            }

            World {
                building,
                step: 0,
                total_agents: configuration.total_agents(),
                gates: Gate::load_gates(&configuration.venue_tags.gates_info),
                mouths: Mouth::load_mouths(&configuration.venue_tags.mouths_info),
            }
        }

        // Insert agent into new layer, random posistion on door
        pub fn insert_agent(&mut self, agent: &mut model::Agent, rng: &mut ThreadRng) {
            let floor = self.building.get_mut(&agent.layer).expect(""); // This cannot fail anyways

            let gate_position = if let Some(gate) = self.gates.get(&agent.init) {
                gate
            } else {    // Not a valid gate
                return;
            };

            floor.place_agent(agent, gate_position.choose(rng).unwrap().to_owned());
        }

        // Move agent to new position
        pub fn step_agent(&mut self, agent: &mut model::Agent) {
            let floor = self.building.get_mut(&agent.layer).expect(""); // This cannot fail anyways
            let position = agent.action_movement().to_owned();

            floor.place_agent(agent, position);
        }
    }
}
