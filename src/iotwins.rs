pub mod model {

    use rand::prelude::ThreadRng;
    use serde::Deserialize;
    use std::collections::HashMap;

    use crate::iotwins::world::World;

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

    use indicatif::{ProgressBar, ProgressStyle, ProgressIterator};
    use rayon::iter::{IntoParallelRefIterator, ParallelIterator};
    use serde::{Deserialize, Serialize};
    use std::{
        collections::{BinaryHeap, HashMap, HashSet},
        iter::zip,
    };

    use crate::{
        config::configuration::Parameters,
        engine::matrix::Matrix,
        engine::matrix::Position,
        iotwins::{
            self,
            model::{self, Arrival},
            world,
        },
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

    #[derive(Debug, Serialize, Deserialize)]
    pub struct MapJump {
        location: Vec<usize>,
        position: Position,
        conexions: HashMap<String, Vec<usize>>, // Floor -> location
    }

    impl MapJump {
        pub fn find_location(building: World) -> HashMap<String, Vec<MapJump>> {
            let points: HashSet<u8> = HashSet::from([10, 11, 3]);

            // This will define the position of each facility
            let mut jumps: HashMap<String, HashMap<u8, Vec<Vec<usize>>>> = HashMap::new();
            let mut central: HashMap<String, HashMap<u8, Vec<Position>>> = HashMap::new();

            print!("[INFO] Detecting structures\t");

            // Generate nodes
            for (layer, floor) in building.building {
                let mut already_visited: HashSet<usize> =
                    HashSet::with_capacity(floor.blueprint.data.len());

                let mut elevators: Vec<Vec<usize>> = Vec::new();
                let mut up_stairs: Vec<Vec<usize>> = Vec::new();
                let mut down_stairs: Vec<Vec<usize>> = Vec::new();

                let mut elevators_pos: Vec<Position> = Vec::new();
                let mut up_stairs_pos: Vec<Position> = Vec::new();
                let mut down_stairs_pos: Vec<Position> = Vec::new();

                let simple_ground = floor.ground_truth.data.clone();

                // Progress bar (CUTE and no performance impact)
                let progress_bar =
                    ProgressBar::new(floor.ground_truth.data.len().try_into().unwrap());

                progress_bar.set_message(layer.to_string());

                progress_bar.set_style(
                    ProgressStyle::default_spinner()
                        .template(
                            "{spinner} {msg} {elapsed_precise} {bar:20} {pos}/{len} [{percent}% - {eta_precise}]",
                        ).progress_chars("#>#-"));
                // End of progress bar stle

                // Simplified version of the blueprints its used, based on GT layer
                simple_ground
                    .into_iter()
                    .progress_with(progress_bar)
                    .enumerate()
                    .for_each(|(position, value)| {
                        if value != 255
                            && !already_visited.contains(&position)
                            && points.contains(&value)
                        {
                            let (facility, mov) = iotwins::world::MapJump::find_structure(
                                &floor.ground_truth.data,
                                position,
                                value,
                            );

                            let pos: Vec<Position> = facility
                                .par_iter()
                                .map(|p| Position {
                                    x: (p / 627) as i32,
                                    y: (p % 627) as i32,
                                })
                                .collect();

                            already_visited.extend(facility.clone());

                            match mov {
                                10 => {
                                    down_stairs.push(facility);
                                    down_stairs_pos.push(Position::middle(pos));
                                }
                                11 => {
                                    up_stairs.push(facility);
                                    up_stairs_pos.push(Position::middle(pos));
                                }
                                _ => {
                                    elevators.push(facility);
                                    elevators_pos.push(Position::middle(pos));
                                }
                            }
                        }
                    });

                // Store results for later conexion
                jumps.insert(
                    layer.to_string(),
                    HashMap::from([(10, down_stairs), (11, up_stairs), (3, elevators)]),
                );

                central.insert(
                    layer,
                    HashMap::from([
                        (10, down_stairs_pos),
                        (11, up_stairs_pos),
                        (3, elevators_pos),
                    ]),
                );
            }

            let layers: Vec<String> = central.clone().into_iter().map(|(key, _)| key).collect();

            let mut building_mapping: HashMap<String, Vec<MapJump>> = HashMap::new();

            // Progress bar (CUTE and no performance impact)
            let progress_bar = ProgressBar::new(layers.len().try_into().unwrap());

            progress_bar.set_style(
                    ProgressStyle::default_spinner()
                        .template(
                            "{spinner} {elapsed_precise} {bar:20} {pos}/{len} [{percent}% - {eta_precise}]",
                        ).progress_chars("#>#-"));
            // End of progress bar stle

            // Get all conexions for each jump
            for (layer, position_hash) in central.iter().progress_with(progress_bar) {
                // Get all other layers
                let mut destination_layers: Vec<String> = layers.clone();
                destination_layers.retain(|l| l != layer);

                // Store jumps for each layer
                let mut layer_vec: Vec<MapJump> = Vec::new();

                let layer_jumps = jumps.get(layer).expect("[ERROR]");

                for (jump_type, positions) in position_hash {
                    match jump_type {
                        10 => {
                            let locations = layer_jumps.get(&11).expect("[ERROR]");

                            for (position, location) in zip(positions, locations) {
                                let mut conexions: HashMap<String, Vec<usize>> = HashMap::new();

                                for destination in &destination_layers {
                                    let possible_links = central
                                        .get(destination)
                                        .expect("[ERROR]")
                                        .get(&11)
                                        .expect("[ERROR]");

                                    let possible_groups = jumps
                                        .get(destination)
                                        .expect("[ERROR]")
                                        .get(&11)
                                        .expect("[ERROR]");

                                    if !possible_groups.is_empty() && !possible_links.is_empty() {
                                        let closest_idx =
                                            Position::closest(position, possible_links.to_vec());

                                        let group = &possible_groups[closest_idx];

                                        conexions.insert(destination.to_string(), group.to_vec());
                                    }
                                }

                                layer_vec.push(MapJump {
                                    location: location.to_vec(),
                                    position: *position,
                                    conexions,
                                });
                            }
                        }
                        11 => {
                            let locations = layer_jumps.get(&10).expect("[ERROR]");

                            for (position, location) in zip(positions, locations) {
                                let mut conexions: HashMap<String, Vec<usize>> = HashMap::new();

                                for destination in &destination_layers {
                                    let possible_links = central
                                        .get(destination)
                                        .expect("[ERROR]")
                                        .get(&10)
                                        .expect("[ERROR]");

                                    let possible_groups = jumps
                                        .get(destination)
                                        .expect("[ERROR]")
                                        .get(&10)
                                        .expect("[ERROR]");

                                    if !possible_groups.is_empty() && !possible_links.is_empty() {
                                        let closest_idx =
                                            Position::closest(position, possible_links.to_vec());

                                        let group = &possible_groups[closest_idx];

                                        conexions.insert(destination.to_string(), group.to_vec());
                                    }
                                }

                                layer_vec.push(MapJump {
                                    location: location.to_vec(),
                                    position: *position,
                                    conexions,
                                });
                            }
                        }
                        _ => {
                            let locations = layer_jumps.get(&3).expect("[ERROR]");

                            for (position, location) in zip(positions, locations) {
                                let mut conexions: HashMap<String, Vec<usize>> = HashMap::new();

                                for destination in &destination_layers {
                                    let possible_links = central
                                        .get(destination)
                                        .expect("[ERROR]")
                                        .get(&3)
                                        .expect("[ERROR]");

                                    let possible_groups = jumps
                                        .get(destination)
                                        .expect("[ERROR]")
                                        .get(&3)
                                        .expect("[ERROR]");

                                    if !possible_groups.is_empty() && !possible_links.is_empty() {
                                        let closest_idx =
                                            Position::closest(position, possible_links.to_vec());

                                        let group = &possible_groups[closest_idx];

                                        conexions.insert(destination.to_string(), group.to_vec());
                                    }
                                }

                                layer_vec.push(MapJump {
                                    location: location.to_vec(),
                                    position: *position,
                                    conexions,
                                });
                            }
                        }
                    }
                }

                building_mapping.insert(layer.to_string(), layer_vec);
            }

            println!("[DONE]");

            building_mapping
        }

        fn contiguous(position: usize) -> Vec<usize> {
            let col_pos = position % 627;
            let row_pos = position / 627;

            match row_pos {
                0 => {
                    // First row
                    Vec::from([
                        627 * (row_pos + 1) + col_pos + 1,
                        627 * (row_pos + 1) + col_pos - 1,
                        627 * (row_pos + 1) + col_pos,
                        627 * row_pos + col_pos - 1,
                        627 * row_pos + col_pos + 1,
                    ])
                }
                627 => {
                    // Last row
                    Vec::from([
                        627 * row_pos + col_pos - 1,
                        627 * row_pos + col_pos + 1,
                        627 * (row_pos - 1) + col_pos - 1,
                        627 * (row_pos - 1) + col_pos,
                        627 * (row_pos - 1) + col_pos + 1,
                    ])
                }
                _ => Vec::from([
                    // Any other row
                    627 * (row_pos + 1) + col_pos + 1,
                    627 * (row_pos + 1) + col_pos - 1,
                    627 * (row_pos + 1) + col_pos,
                    627 * row_pos + col_pos - 1,
                    627 * row_pos + col_pos + 1,
                    627 * (row_pos - 1) + col_pos - 1,
                    627 * (row_pos - 1) + col_pos,
                    627 * (row_pos - 1) + col_pos + 1,
                ]),
            }
        }

        fn find_structure(floor: &[u8], position: usize, x: u8) -> (Vec<usize>, u8) {
            let mut facility: HashSet<usize> = HashSet::from([position]);

            let mut surroundings = BinaryHeap::from(iotwins::world::MapJump::contiguous(position));

            while let Some(pos) = surroundings.pop() {
                // If same structure
                if floor[pos] == x && !facility.contains(&pos) {
                    facility.insert(pos);
                    // Add contiguous positions
                    iotwins::world::MapJump::contiguous(position)
                        .iter()
                        .for_each(|new_pos| surroundings.push(*new_pos));
                }
            }

            (facility.into_iter().collect(), x)
        }
    }

    #[derive(Debug, Clone)]
    struct Floor {
        name: String,
        blueprint: Matrix<u8>,
        agents: Matrix<u64>,
        pub ground_truth: Matrix<u8>,
    }

    impl Floor {
        fn load_floor(floor: String, path: String, size: (u32, u32)) -> Floor {
            let blueprint = Matrix::load_layer(&path);

            Floor {
                name: floor,
                ground_truth: Floor::ground_truth(&blueprint),
                blueprint,
                agents: Matrix::new(size),
            }
        }

        fn ground_truth(blueprint: &Matrix<u8>) -> Matrix<u8> {
            Matrix::ground_thruth(
                blueprint,
                HashMap::from([
                    // Walls
                    (84, 1),  // Obstacles
                    (202, 1), // No transit
                    (159, 1), // Kitchen
                    (7, 1),   // Technical staff
                    (241, 1), // Lockers
                    (103, 1), // SAT
                    (131, 1), // Men restroom
                    (118, 1), // Women restroom
                    (164, 1), // Restroom
                    (122, 1), // Bar
                    // Map jumps
                    (109, 10), // Down stair
                    (232, 11), // Up stair
                    (112, 20), // Down ramp
                    (189, 21), // Up ramp
                    (182, 3),  // Elevator
                ]),
            )
        }
    }

    #[derive(Debug)]
    pub struct World {
        building: HashMap<String, Floor>,
        step: u64,
        pub gates: HashMap<String, Vec<usize>>,
        pub mouths: HashMap<String, HashMap<i16, Vec<usize>>>,
        pub arrivals: HashMap<i32, Vec<Arrival>>,
    }

    impl World {
        // pub fn default_paths(&self) -> ConcurrentHashMap {
        //     // TODO: Compare all gates against each other, one search for two routes
        //     let routes = engine::routes::ConcurrentHashMap::new();

        //     for (level, floor) in &self.building {
        //         if !self.mouths.contains_key(level) {
        //             continue;
        //         } // If level has no mouths continue

        //         println!("[INFO] Finding routes for layer {level}");

        //         let floor_gt = &floor.ground_truth.data;

        //         let mut all_mouths: Vec<Vec<usize>> = self
        //             .mouths
        //             .get(level)
        //             .expect("[ERROR] Layer does not exist")
        //             .clone()
        //             .into_values()
        //             .collect();

        //         let mouths = self
        //             .mouths
        //             .get(level)
        //             .expect("[ERROR] Mouth does not exist");

        //         // Progress bar (CUTE and no performance impact)
        //         let progress_bar = ProgressBar::new(mouths.values().len().try_into().unwrap());

        //         progress_bar.set_style(
        //             ProgressStyle::default_spinner()
        //                 .template(
        //                     "{elapsed:.white} {bar:25.blue/white} {pos:.blue}/{len:.white} ({eta_precise})",
        //                 )
        //                 .progress_chars("$>#"),
        //         );
        //         // End of progress bar stle

        //         for positions in mouths.values().progress_with(progress_bar) {
        //             all_mouths.retain(|poss| poss != positions); // Remove current mouth to not be computed

        //             // Every single value that is assigned to a given mouth
        //             for position in positions {
        //                 // All other mouths' positions grouped by its corresponding mouth distributed each group to a thread
        //                 (0..all_mouths.len()).into_par_iter().for_each(|idx| {
        //                     all_mouths[idx].iter().for_each(|destination| {
        //                         // This part is parallelized, multiple paths are comuted at the same time,
        //                         // multiple destination for the same mouth
        //                         let path = engine::path_finding::a_star(
        //                             floor_gt,
        //                             *position,
        //                             *destination,
        //                             627,
        //                         );
        //                         routes.insert(level.to_string(), *position, *destination, path);
        //                     });
        //                 });
        //             }
        //         }
        //     }

        //     routes
        // }
    }

    // Generate a unique HashMap with the whole simulation with index for checkpointing and agents
    pub fn create_world(configuration: Parameters) -> World {
        let floors = configuration.topology.layers();
        let mut building: HashMap<String, Floor> = HashMap::new();

        println!("[INFO] Creating world");

        for (floor, path) in floors {
            let layer = Floor::load_floor(
                floor.to_string(),
                path.to_string(),
                configuration.get_world_size(),
            );

            building.insert(floor.to_string(), layer);
        }

        World {
            building,
            step: 0,
            gates: world::load_gates(configuration.venue_tags.gates_info),
            mouths: world::load_mouths(configuration.venue_tags.mouths_info),
            arrivals: model::load_arrivals(configuration.venue_tags.arrivals_info_csv),
        }
    }
}
