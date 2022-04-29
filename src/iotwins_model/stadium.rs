use crate::{
    engine::matrix::Matrix,
    iotwins_model::{
        routes::{find_route, Route},
        structures::{generate_structures, load_mouths, Structure},
    },
};
use indicatif::{ParallelProgressIterator, ProgressBar, ProgressIterator, ProgressStyle};
use rayon::prelude::*;
use serde::Deserialize;
use std::collections::{HashMap, HashSet};

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
    pub ground_truth: Matrix<u8>,
    pub structures: HashMap<u8, HashSet<Structure>>, // Mapping Position -> matrix by type of structure
    pub structures_paths: HashSet<Route>,
    pub mouths: HashMap<u16, Structure>, // Agent destinations
    pub mouths_paths: HashMap<u16, HashSet<Route>>, // From down-stairs -> mouths (grandstands)
}

impl Floor {
    pub fn create_floor(path: String, name: String) -> Floor {
        let ground_truth = Floor::ground_truth(&Matrix::load_layer(&path));
        let structures = generate_structures(&ground_truth);
        let mouths = load_mouths(&name);

        Floor {
            structures_paths: Floor::stairs_paths(&ground_truth, &structures, &name),
            mouths_paths: Floor::mouth_paths(&ground_truth, &structures, &mouths, &name),
            mouths,
            structures,
            ground_truth,
        }
    }

    pub fn load_floor(
        path: String,
        name: String,
        mouths_paths: HashMap<u16, HashSet<Route>>,
        structures_paths: HashSet<Route>,
    ) -> Floor {
        let mouths = load_mouths(&name);
        let ground_truth = Floor::ground_truth(&Matrix::load_layer(&path));

        Floor {
            structures: generate_structures(&ground_truth),
            ground_truth,
            structures_paths,
            mouths,
            mouths_paths,
        }
    }

    fn stairs_paths(
        gt: &Matrix<u8>,
        structures: &HashMap<u8, HashSet<Structure>>,
        layer: &String,
    ) -> HashSet<Route> {
        let down = structures.get(&10).expect("");
        let up = structures.get(&11).expect("");

        // Progress bar
        let progress_bar = ProgressBar::new(down.len().try_into().unwrap());

        progress_bar.set_message(format!("{layer} - Map jumps"));

        progress_bar.set_style(
            ProgressStyle::default_spinner()
                .template("{spinner} {msg} {bar:10} {pos}/{len}")
                .progress_chars("#>#-"),
        );
        // End of progress bar

        let stairs_paths = down
            .par_iter()
            .progress_with(progress_bar)
            .flat_map(|p1| up.par_iter().map(|p2| find_route(gt, p1, p2)));

        let mut res = HashSet::from_par_iter(stairs_paths);
        res.shrink_to_fit();

        res
    }

    fn mouth_paths(
        gt: &Matrix<u8>,
        structures: &HashMap<u8, HashSet<Structure>>,
        mouths: &HashMap<u16, Structure>,
        layer: &String,
    ) -> HashMap<u16, HashSet<Route>> {
        // Down stairs are the only positions from where you can go to the grandstands, agents will arrive at down-stairs
        let down = structures.get(&10).expect("");
        // For each destination mouth, a set of possible routes from near stairs
        let mut mouths_paths: HashMap<u16, HashSet<Route>> = HashMap::new();

        // Progress bar
        let progress_bar = ProgressBar::new(down.len().try_into().unwrap());

        progress_bar.set_message(format!("{layer} - Mouths paths"));

        progress_bar.set_style(
            ProgressStyle::default_spinner()
                .template("{spinner} {msg} {bar:10} {pos}/{len}")
                .progress_chars("#>#-"),
        );
        // End of progress bar

        mouths
            .iter()
            .progress_with(progress_bar)
            .for_each(|(id, mouth)| {
                let mouth_routes: HashSet<Route> = HashSet::from_par_iter(
                    down.par_iter().map(|stair| find_route(gt, stair, mouth)),
                );

                mouths_paths.insert(*id, mouth_routes);
            });

        // Reduce size
        mouths_paths.shrink_to_fit();

        mouths_paths
    }

    pub fn ground_truth(blueprint: &Matrix<u8>) -> Matrix<u8> {
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

pub mod arrivals {
    use std::collections::HashMap;

    use serde::Deserialize;

    #[derive(Deserialize)]
    struct RawArrival {
        index: usize,
        gate: String,
        mouth: u16,
        minutes_to_game: i32,
        agents: u8,
    }

    #[derive(Debug)]
    pub struct Arrival {
        gate: String,
        mouth: u16,
        agents: u8,
    }

    pub fn load_arrivals() -> HashMap<i32, Vec<Arrival>> {
        let mut arrivals: HashMap<i32, Vec<Arrival>> = HashMap::new();

        let mut reader = csv::Reader::from_path("resources/627/tagging/arrivals.csv")
            .expect("[ERROR] Arrivals file not found");

        for result in reader.deserialize() {
            let record: RawArrival = result.expect("[ERROR] Incorrect gate format");

            match arrivals.entry(record.minutes_to_game) {
                std::collections::hash_map::Entry::Occupied(mut arrivals) => {
                    arrivals.get_mut().push(Arrival {
                        gate: record.gate,
                        mouth: record.mouth,
                        agents: record.agents,
                    });
                }
                std::collections::hash_map::Entry::Vacant(arrivals) => {
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
}
