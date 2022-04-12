pub mod tagging {

    use std::{
        collections::{BinaryHeap, HashMap, HashSet},
        fs::File,
        iter::zip,
    };

    use serde::{Deserialize, Serialize};

    use crate::{
        engine::matrix::{Matrix, Position},
        iotwins_model::stadium::environment::Floor,
    };

    #[derive(Serialize, Deserialize, Clone, PartialEq, Eq)]
    pub struct Jump {
        pub location: Vec<usize>,
        position: Position,
        pub conexions: HashMap<String, Vec<usize>>,
    }

    impl Jump {
        pub fn find_locations(building: &HashMap<String, Floor>) -> HashMap<String, Vec<Jump>> {
            let points: HashSet<u8> = HashSet::from([10, 11, 3]);

            // This will define the position of each facility
            let mut jumps: HashMap<String, HashMap<u8, Vec<Vec<usize>>>> = HashMap::new();
            let mut central: HashMap<String, HashMap<u8, Vec<Position>>> = HashMap::new();

            println!("[INFO] Detecting structures");

            // Find structures
            building.iter().for_each(|(layer, floor)| {
                let mut already_visited: HashSet<usize> =
                    HashSet::with_capacity(floor.blueprint.data.len());

                let mut elevators: Vec<Vec<usize>> = Vec::new();
                let mut up_stairs: Vec<Vec<usize>> = Vec::new();
                let mut down_stairs: Vec<Vec<usize>> = Vec::new();

                let mut elevators_pos: Vec<Position> = Vec::new();
                let mut up_stairs_pos: Vec<Position> = Vec::new();
                let mut down_stairs_pos: Vec<Position> = Vec::new();

                let simple_ground = floor.ground_truth.data.clone();

                // Simplified version of the blueprints its used, based on GT layer
                simple_ground
                    .into_iter()
                    .enumerate()
                    .for_each(|(idx, value)| {
                        if value != 255
                            && !already_visited.contains(&idx)
                            && points.contains(&value)
                        {
                            let (facility, mov) =
                                Jump::find_structure(&floor.ground_truth, idx, value);

                            let pos: Vec<Position> =
                                facility.iter().map(|p| Position::new(*p)).collect();

                            already_visited.extend(facility.to_vec());

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

                // Store results for later conexion (nodes)
                jumps.insert(
                    layer.to_string(),
                    HashMap::from([(10, down_stairs), (11, up_stairs), (3, elevators)]),
                );

                central.insert(
                    layer.to_string(),
                    HashMap::from([
                        (10, down_stairs_pos),
                        (11, up_stairs_pos),
                        (3, elevators_pos),
                    ]),
                );
            });

            let layers: Vec<String> = central.keys().into_iter().map(|k| k.to_string()).collect();

            let mut building_mapping: HashMap<String, Vec<Jump>> = HashMap::new();

            // Link counterpart structures
            central
                .iter()
                .for_each(|(layer, position_hash)| {
                    let structure_hash = jumps.get(layer).expect("");

                    // Get all other layers
                    let destination_layers: Vec<String> = layers
                        .clone()
                        .iter()
                        .filter(|l| l.to_string() != *layer)
                        .map(|l| l.to_string())
                        .collect();

                    // Store jumps for each layer
                    let mut layer_vec: Vec<Jump> = Vec::new();

                    position_hash.iter().for_each(|(jump_type, positions)| {
                        let structures = structure_hash.get(jump_type).expect("");

                        match jump_type {
                            10 => {
                                zip(positions, structures).into_iter().for_each(
                                    |(position, location)| {
                                        let mut conexions: HashMap<String, Vec<usize>> =
                                            HashMap::new();

                                        destination_layers.iter().for_each(|layer| {
                                            let possible_positions =
                                                central.get(layer).expect("").get(&11).expect("");
                                            let possible_locations =
                                                jumps.get(layer).expect("").get(&11).expect("");

                                            if !possible_positions.is_empty()
                                                && !possible_locations.is_empty()
                                            {
                                                match Position::closest(
                                                    position,
                                                    possible_positions.to_vec(),
                                                ) {
                                                    Some(idx) => conexions.insert(
                                                        layer.to_string(),
                                                        possible_locations[idx].to_vec(),
                                                    ),
                                                    None => None,
                                                };
                                            }
                                        });

                                        layer_vec.push(Jump {
                                            location: location.to_vec(),
                                            position: *position,
                                            conexions,
                                        });
                                    },
                                );
                            }
                            11 => {
                                zip(positions, structures).into_iter().for_each(
                                    |(position, location)| {
                                        let mut conexions: HashMap<String, Vec<usize>> =
                                            HashMap::new();

                                        destination_layers.iter().for_each(|layer| {
                                            let possible_positions =
                                                central.get(layer).expect("").get(&10).expect("");
                                            let possible_locations =
                                                jumps.get(layer).expect("").get(&10).expect("");

                                            if !possible_positions.is_empty()
                                                && !possible_locations.is_empty()
                                            {
                                                match Position::closest(
                                                    position,
                                                    possible_positions.to_vec(),
                                                ) {
                                                    Some(idx) => conexions.insert(
                                                        layer.to_string(),
                                                        possible_locations[idx].to_vec(),
                                                    ),
                                                    None => None,
                                                };
                                            }
                                        });

                                        layer_vec.push(Jump {
                                            location: location.to_vec(),
                                            position: *position,
                                            conexions,
                                        });
                                    },
                                );
                            }
                            _ => {
                                zip(positions, structures).into_iter().for_each(
                                    |(position, location)| {
                                        let mut conexions: HashMap<String, Vec<usize>> =
                                            HashMap::new();

                                        destination_layers.iter().for_each(|layer| {
                                            let possible_positions =
                                                central.get(layer).expect("").get(&3).expect("");
                                            let possible_locations =
                                                jumps.get(layer).expect("").get(&3).expect("");

                                            if !possible_positions.is_empty()
                                                && !possible_locations.is_empty()
                                            {
                                                match Position::closest(
                                                    position,
                                                    possible_positions.to_vec(),
                                                ) {
                                                    Some(idx) => conexions.insert(
                                                        layer.to_string(),
                                                        possible_locations[idx].to_vec(),
                                                    ),
                                                    None => None,
                                                };
                                            }
                                        });

                                        layer_vec.push(Jump {
                                            location: location.to_vec(),
                                            position: *position,
                                            conexions,
                                        });
                                    },
                                );
                            }
                        }
                    });

                    if !layer_vec.is_empty() {
                        building_mapping.insert(layer.to_string(), layer_vec);
                    }
                });

            building_mapping
        }

        // Indicate folder location only
        pub fn save_locations(locations: &HashMap<String, Vec<Jump>>, path: String) {
            locations.iter().for_each(|(layer, locations)| {
                let file_path = format!("{path}-{layer}.json");

                let file = File::create(file_path).expect("[ERROR] No write permissions");

                serde_json::to_writer(file, locations).expect("[ERROR] No file");
            });

            println!("[INFO] Locations saved");
        }

        pub fn load_locations() -> HashMap<String, Vec<Jump>> {
            todo!()
        }

        fn find_structure(floor: &Matrix<u8>, position: usize, x: u8) -> (Vec<usize>, u8) {
            let mut facility: HashSet<usize> = HashSet::from([position]);

            let mut surroundings = BinaryHeap::from(floor.contiguous(position));

            while let Some(pos) = surroundings.pop() {
                // If same structure
                // TODO: add Index & IndexMut traits
                if floor.data[pos] == x && !facility.contains(&pos) {
                    facility.insert(pos);
                    // Add contiguous positions
                    floor
                        .contiguous(position)
                        .iter()
                        .for_each(|new_pos| surroundings.push(*new_pos));
                }
            }

            (facility.into_iter().collect(), x)
        }
    }
}
