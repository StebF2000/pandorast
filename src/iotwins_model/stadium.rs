use crate::engine::matrix::Matrix;
use serde::Deserialize;
use std::collections::HashMap;

#[derive(Deserialize)]
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

#[derive(Clone)]
pub struct Floor {
    name: String,
    pub blueprint: Matrix<u8>,
    agents: Matrix<u64>,
    pub ground_truth: Matrix<u8>,
}

impl Floor {
    pub fn load_floor(floor: String, path: String, size: (usize, usize)) -> Floor {
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
