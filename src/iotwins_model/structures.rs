use crate::engine::matrix::{Matrix, Position};
use serde::{Deserialize, Serialize};
use std::{
    collections::{hash_map::Entry, HashMap, HashSet},
    hash::{Hash, Hasher},
    iter::zip,
};

use rayon::prelude::*;

#[derive(Serialize, Deserialize, Eq, Clone, Default, Debug)]
pub struct Structure {
    pub position: Position,
    pub location: Vec<usize>,
}

impl PartialEq for Structure {
    fn eq(&self, other: &Self) -> bool {
        self.position == other.position
    }
}

impl Hash for Structure {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.position.hash(state);
    }
}

impl Structure {
    /// CAUTION: This function computes a^2 + b^2
    fn distance(&self, other: &Structure) -> i32 {
        self.position.distance(&other.position)
    }

    // Order search space from bigger to smaller distance, better for later pop
    pub fn get_closest_structure(&self, search_space: &[Structure]) -> Option<Structure> {
        // Returns the closest structure, computing euclidean distance in parallel between them all
        search_space
            .into_par_iter()
            .map(|structure| (structure, self.distance(structure)))
            .min_by_key(|(_, dist)| *dist)
            .map(|(structure, _)| structure.to_owned())
    }
}

pub fn generate_structures(ground_truth: &Matrix<u8>) -> HashMap<u8, HashSet<Structure>> {
    let mut visited = HashSet::with_capacity(ground_truth.n_rows * ground_truth.n_rows);

    // let mut elevators: Vec<Vec<usize>> = Vec::new();
    let mut up_stairs: Vec<Vec<usize>> = Vec::new();
    let mut down_stairs: Vec<Vec<usize>> = Vec::new();

    let mut up_stairs_pos: Vec<Position> = Vec::new();
    let mut down_stairs_pos: Vec<Position> = Vec::new();

    for (idx, value) in ground_truth.data.iter().enumerate() {
        // Skip already visited locations and useless positions
        if visited.contains(&idx) || !HashSet::from([10, 11]).contains(value) {
            continue;
        }

        let facility = find_structure(ground_truth, idx, &mut visited);

        match value {
            // Down-stairs
            10 => {
                down_stairs_pos.push(Position::middle_location(&facility, ground_truth.n_rows));
                down_stairs.push(facility);
            }
            // Up-stairs
            11 => {
                up_stairs_pos.push(Position::middle_location(&facility, ground_truth.n_rows));
                up_stairs.push(facility);
            }

            _ => (),
        }
    }

    // Generate paths
    let down_stair: HashSet<Structure> = HashSet::from_iter(
        zip(down_stairs, down_stairs_pos)
            .map(|(location, position)| Structure { position, location }),
    );

    let up_stair: HashSet<Structure> = HashSet::from_iter(
        zip(up_stairs, up_stairs_pos).map(|(location, position)| Structure { position, location }),
    );

    // Reduce size as much as posible
    let mut relation = HashMap::from([(10, down_stair), (11, up_stair)]);
    relation.shrink_to_fit();

    relation
}

#[derive(Deserialize)]
struct RawMouth {
    mouth: String,
    layer: String,
    x: usize,
    y: usize,
}

pub fn load_mouths(layer: &str) -> HashMap<u16, Structure> {
    let mut mouths: HashMap<u16, Vec<usize>> = HashMap::new();

    let mut reader = csv::Reader::from_path("resources/tagging/mouths.csv")
        .expect("[ERROR] Mouths file not found");

    for result in reader.deserialize() {
        let record: RawMouth = result.expect("[ERROR] Incorrect mouth format");

        // Filter correct layer
        if record.layer != *layer {
            continue;
        }

        // If mouth is not present is created, otherwise pushes the new location
        record
            .mouth
            .split('-')
            .map(|s| s.parse::<u16>().unwrap())
            .for_each(|mouth| match mouths.entry(mouth) {
                Entry::Occupied(mut location) => {
                    location.get_mut().push(627 * record.x + record.y);
                }
                Entry::Vacant(location) => {
                    location.insert(vec![627 * record.x + record.y]);
                }
            });
    }

    let data = mouths.into_iter().map(|(id, location)| {
        (
            id,
            Structure {
                position: Position::middle_location(&location, 627),
                location: location.to_vec(),
            },
        )
    });

    // Reduce size
    let mut m = HashMap::from_iter(data);
    m.shrink_to_fit();
    m
}

#[derive(Deserialize)]
struct RawGate {
    layer: String,
    gate: String,
    x: usize,
    y: usize,
}

#[derive(Eq, Clone, Serialize, Deserialize)]
pub struct Gate {
    pub floor: String,
    pub name: String,
    pub structure: Structure,
}

impl PartialEq for Gate {
    fn eq(&self, other: &Self) -> bool {
        self.name == other.name
    }
}

impl Hash for Gate {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.name.hash(state);
    }
}

impl Default for Gate {
    fn default() -> Self {
        Gate {
            floor: String::from(""),
            name: String::from(""),
            structure: Default::default(),
        }
    }
}

/// HashMap of initial points (Gates). Key => usize position on matrix PB
pub fn load_gates() -> HashSet<Gate> {
    let mut gates: HashMap<String, HashMap<String, Vec<usize>>> = HashMap::new();

    let mut reader = csv::Reader::from_path("resources/tagging/gates.csv")
        .expect("[ERROR] Gates file not found");

    // HashMap of initial points (Gates). Key => usize position on matrix PB
    for result in reader.deserialize() {
        let record: RawGate = result.expect("[ERROR] Incorrect gate format");

        match gates.get_mut(&record.layer) {
            Some(gates) => match gates.get_mut(&record.gate) {
                Some(location) => location.push(627 * record.x + record.y),
                None => {
                    gates.insert(record.gate, vec![627 * record.x + record.y]);
                }
            },
            None => {
                let mut data = HashMap::from([(record.gate, vec![627 * record.x + record.y])]);
                data.shrink_to_fit();

                gates.insert(record.layer, data);
            }
        }
    }

    let mut data = HashSet::new();

    gates.iter().for_each(|(layer, gates)| {
        gates.iter().for_each(|(id, location)| {
            data.insert(Gate {
                floor: layer.to_string(),
                name: id.to_string(),
                structure: Structure {
                    position: Position::middle_location(location, 627),
                    location: location.to_vec(),
                },
            });
        });
    });

    // Reduce size
    data.shrink_to_fit();
    data
}

fn find_structure(
    ground_truth: &Matrix<u8>,
    position: usize,
    visited: &mut HashSet<usize>,
) -> Vec<usize> {
    let x = ground_truth.data[position]; // Structure identifier

    // This way there are no duplicate positions
    let mut facility: HashSet<usize> = HashSet::from([position]);

    let mut surroundings: Vec<usize> = ground_truth.contiguous(position); // Contiguous returns the 8 surrounding positions

    while let Some(pos) = surroundings.pop() {
        // If same structure do not visit again

        // Check for the same type of structure
        if ground_truth.data[pos] == x {
            visited.insert(pos);
            facility.insert(pos);

            let next_positions: Vec<usize> = ground_truth
                .contiguous(pos)
                .into_iter()
                .filter(|pos| !visited.contains(pos))
                .collect();

            surroundings.extend(next_positions);
        }
    }

    facility.into_iter().collect()
}
