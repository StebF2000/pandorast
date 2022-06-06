use std::{
    collections::{HashMap, HashSet},
    fs::{read_dir, File, ReadDir},
    io::{BufReader, BufWriter},
    time::Instant,
};

use bincode::{deserialize_from, serialize_into};
use indicatif::{ProgressBar, ProgressIterator, ProgressStyle};
use rand::distributions::Uniform;
use rayon::prelude::*;
use serde::{Deserialize, Serialize};

use crate::{
    config::configuration::Parameters,
    iotwins_model::{
        arrivals::{load_arrivals, Arrival},
        routes::{find_route, Route},
        snapshot::Snapshot,
        stadium::{self, Floor},
        structures::{load_gates, Gate, Structure},
    },
};

#[derive(Serialize, Deserialize)]
pub struct World {
    pub building: HashMap<String, stadium::Floor>,
    pub building_conexions: HashMap<String, HashMap<Structure, HashMap<String, Structure>>>,
    pub step: u32,
    pub agent_count: usize,
    pub arrivals: HashMap<i32, Vec<Arrival>>,
    pub gates: HashSet<Gate>,
    pub gates_to_stairs: HashMap<Gate, HashSet<Route>>,
    pub gates_to_mouths: HashMap<Gate, HashMap<u16, Route>>,
}

impl World {
    // Returns closest structure with exit to the desired layer
    pub fn get_closest_conexion(
        &self,
        origen: &Structure,
        floor: &String,
        destination_floor: &String,
    ) -> Option<Structure> {
        let level = self.building_conexions.get(floor).expect("");

        // Get only stairs connecting to destination_floor
        let search_space: Vec<Structure> = level
            .iter()
            .filter_map(
                |(structure, conexions)| match conexions.contains_key(destination_floor) {
                    true => Some(structure.to_owned()),
                    false => None,
                },
            )
            .collect();

        // Return closest structure
        origen.get_closest_structure(&search_space)
    }

    pub fn evolve(&mut self, interest: Uniform<f32>) {
        // 0.3s per step, 200 steps make a minute. Agent arrival only executed once per minute
        if self.step % 200 == 0 {
            let total_added = self.agent_arrival(interest); // Agent arrivals
            self.snapshot().write_snapshot(); // Save snapshot

            println!(
                "[SIM] T {}. Total agents: {} ({total_added} added)",
                self.get_time(),
                self.count_agents()
            )
        }

        self.step += 1;
    }

    // Returns the number of agents added to the simulation env.
    fn agent_arrival(&mut self, interest: Uniform<f32>) -> usize {
        let time: i32 = self.get_time();
        let mut total_added = 0;

        // 1. Insert new agents into the simulation
        // 1.1 Check if agents should be inserted
        if let Some(arrivals) = self.arrivals.to_owned().get(&time) {
            arrivals.iter().for_each(|arrival| {
                if let Some(gate) = self.gates.to_owned().get(&Gate {
                    name: arrival.gate.to_string(),
                    ..Default::default()
                }) {
                    match self.get_closest_conexion(
                        &gate.structure,
                        &gate.floor,
                        &arrival.mouth_layer(),
                    ) {
                        Some(target) => {
                            // Remove non-listed gates
                            let agents = arrival.generate_agents(
                                target.to_owned(),
                                self.agent_count,
                                interest,
                            );

                            let floor = self.building.get_mut(&gate.floor).unwrap();
                            // This include non-inserted agents
                            self.agent_count += agents.len();

                            // Insert agents on gate layer (floor)
                            if let Some(route) =
                                self.gates_to_mouths.get(gate).unwrap().get(&arrival.mouth)
                            {
                                // Destination mouth same layer than gate
                                total_added += floor.insert_agents(&agents, route.to_owned());
                            } else if let Some(route) =
                                self.gates_to_stairs.get(gate).unwrap().get(&Route {
                                    // Agent towards another layer
                                    origin: gate.structure.to_owned(),
                                    destination: target,
                                    ..Default::default()
                                })
                            {
                                total_added += floor.insert_agents(&agents, route.to_owned());
                            }
                        }
                        None => {}
                    }
                }
            });
        }

        total_added
    }

    fn get_time(&mut self) -> i32 {
        (self.step as i32 / 200) - 90
    }

    fn snapshot(&self) -> Snapshot {
        let building = self
            .building
            .iter()
            .map(|(layer, floor)| (layer.to_string(), floor.generate_save()));

        Snapshot {
            iter: self.step,
            building: HashMap::from_iter(building),
        }
    }

    pub fn save_structures(&self) {
        println!("[INFO] Saving structures...");
        let mut data = HashMap::new();

        for (layer, floor) in &self.building {
            data.insert(layer.to_string(), floor.structures.to_owned());
        }

        let file1 = File::create("resources/stairs.json").expect(""); // All structures by layer
        serde_json::to_writer_pretty(file1, &data).expect("");
    }

    pub fn save_layer_paths(&self) {
        println!("[INFO] Saving paths...");

        // All routes in layer
        for (layer, floor) in &self.building {
            let file1 = File::create(format!("resources/stairs_paths/{layer}.json")).expect("");
            let file2 = File::create(format!("resources/mouths_paths/{layer}.json")).expect("");

            let data1: Vec<Route> = floor.structures_paths.iter().cloned().collect();

            serde_json::to_writer_pretty(file1, &data1).expect("");
            serde_json::to_writer_pretty(file2, &floor.mouths_paths).expect("");
        }
    }

    // HPC environment saving (Who cares about humans)
    pub fn bincode_save(&self) {
        let start = Instant::now();
        // Save building completely

        let mut file = BufWriter::new(File::create("resources/IoTwins.bin").unwrap());

        serialize_into(&mut file, &self).unwrap();

        println!("[INFO] Time elapsed: {:?}", start.elapsed());
    }

    // Paths between gates and stairs in layer
    fn gates_stairs(
        building: &HashMap<String, stadium::Floor>,
        gates: &HashSet<Gate>,
    ) -> HashMap<Gate, HashSet<Route>> {
        // Progress bar
        let progress_bar = ProgressBar::new(gates.len().try_into().unwrap());

        progress_bar.set_message("Gate -> Stairs".to_string());

        progress_bar.set_style(
            ProgressStyle::default_spinner()
                .template("{msg} {bar:10} {pos}/{len}")
                .progress_chars("#>#-"),
        );
        // End of progress bar

        let routes = gates.iter().progress_with(progress_bar).map(|gate| {
            let floor = building.get(&gate.floor).expect("");
            let up_stairs = floor.structures.get(&11).expect("");

            let stairs_paths = up_stairs
                .par_iter()
                .filter_map(|p2| find_route(&floor.ground_truth, &gate.structure, p2));

            (gate.to_owned(), HashSet::from_par_iter(stairs_paths))
        });

        HashMap::from_iter(routes)
    }

    // Paths between gates and up-stairs in layer
    fn gates_mouths(
        building: &HashMap<String, stadium::Floor>,
        gates: &HashSet<Gate>,
    ) -> HashMap<Gate, HashMap<u16, Route>> {
        // Progress bar
        let progress_bar = ProgressBar::new(gates.len().try_into().unwrap());

        progress_bar.set_message("Gate -> Mouths".to_string());

        progress_bar.set_style(
            ProgressStyle::default_spinner()
                .template("{msg} {bar:10} {pos}/{len}")
                .progress_chars("#>#-"),
        );
        // End of progress bar

        let routes = gates.iter().map(|gate| {
            let floor = building.get(&gate.floor).expect("");

            let gate_routes = floor.mouths.par_iter().filter_map(|(id, mouth)| {
                find_route(&floor.ground_truth, &gate.structure, mouth).map(|route| (*id, route))
            });

            (gate.to_owned(), HashMap::from_par_iter(gate_routes))
        });

        HashMap::from_iter(routes)
    }

    pub fn count_agents(&self) -> usize {
        self.building
            .iter()
            .map(|(_, floor)| floor.agents.len())
            .sum()
    }

    // For each structure in floor gets their destination (Links stairs between layers). Generates proper global structure between them all
    fn connect_structures(
        building: &HashMap<String, stadium::Floor>,
    ) -> HashMap<String, HashMap<Structure, HashMap<String, Structure>>> {
        let conexions = building.iter().map(|(layer, floor)| {
            let up_structures = floor.structures.get(&11).expect("");

            // Progress bar
            let progress_bar = ProgressBar::new(up_structures.len().try_into().unwrap());

            progress_bar.set_message(format!("{layer} - Up stairs"));

            progress_bar.set_style(
                ProgressStyle::default_spinner()
                    .template("{spinner} {msg} {bar:10} {pos}/{len}")
                    .progress_chars("#>#-"),
            );

            let layer_stairs = up_structures
                .iter()
                .progress_with(progress_bar)
                .map(|structure| {
                    let structure_exits = building
                        .par_iter()
                        .filter(|(l, _)| l.to_string() != *layer) // Remove current floor
                        .filter_map(|(destination_layer, destination_floor)| {
                            let search_space = destination_floor.structures.get(&10).expect("");

                            structure
                                .get_closest_structure(&Vec::from_iter(search_space.to_owned()))
                                .map(|structure| (destination_layer.to_string(), structure))
                        });

                    (
                        structure.to_owned(),
                        HashMap::from_par_iter(structure_exits),
                    )
                });

            (layer.to_string(), HashMap::from_iter(layer_stairs))
        });

        HashMap::from_iter(conexions)
    }

    /// Method for floor loading
    fn get_floor_file(dir: ReadDir, layer: &str) -> String {
        let p: Vec<String> = dir
            .into_iter()
            .filter_map(|file_path| {
                match file_path
                    .as_ref()
                    .unwrap()
                    .file_name()
                    .to_str()
                    .unwrap()
                    .to_owned()
                    .split('.')
                    .collect::<Vec<&str>>()[0]
                    == layer
                {
                    true => Some(file_path.unwrap().path().to_str().unwrap().to_owned()),
                    false => None,
                }
            })
            .collect();

        p[0].to_owned()
    }
}

// Generate a unique HashMap with the whole simulation with index for checkpointing and agents
pub fn create_world(configuration: Parameters) -> World {
    let floors = configuration.topology.layers();
    let mut building: HashMap<String, stadium::Floor> = HashMap::new();

    println!("[INFO] Creating world");
    let start = Instant::now();

    floors.into_iter().for_each(|(floor, path)| {
        let mut layer = stadium::Floor::create_floor(path, floor.to_string());

        building.insert(floor.to_string(), layer);
    });

    let w = World {
        step: 0,
        agent_count: 0,
        building_conexions: World::connect_structures(&building),
        gates: load_gates(),
        gates_to_stairs: World::gates_stairs(&building, &load_gates()),
        gates_to_mouths: World::gates_mouths(&building, &load_gates()),
        arrivals: load_arrivals(),
        building,
    };

    println!("[INFO] Environment created [{:?}]", start.elapsed());

    w
}

pub fn load_world(
    structures_path: String,
    structures_paths_dir: String,
    mouths_paths_dir: String,
    configuration: &Parameters,
) -> World {
    println!("[INFO] Loading world...");

    // Use configuration to load in parallel the floors for the building.

    let floors = configuration.topology.layers();

    let structures_reader = File::open(structures_path).unwrap();
    let structures: HashMap<String, HashMap<u8, HashSet<Structure>>> =
        serde_json::from_reader(BufReader::new(structures_reader)).unwrap();

    let start = Instant::now();

    // Load data in parallel
    let building_raw = floors.into_par_iter().map(|(floor, path)| {
        // Get correct layer file
        let structures_path =
            World::get_floor_file(read_dir(&structures_paths_dir).unwrap(), floor);
        let mouths_path = World::get_floor_file(read_dir(&mouths_paths_dir).unwrap(), floor);

        // Load data
        let floor_structures = structures.get(floor).unwrap().to_owned();
        let floor_structures_paths_raw: Vec<Route> =
            serde_json::from_reader(BufReader::new(File::open(structures_path).unwrap())).unwrap(); // Convert to HashSet

        let floor_structures_paths: HashSet<Route> = HashSet::from_iter(floor_structures_paths_raw);

        let floor_mouths_paths: HashMap<u16, HashSet<Route>> =
            serde_json::from_reader(BufReader::new(File::open(mouths_path).unwrap())).unwrap();

        let layer = Floor::load_floor(
            &path,
            floor,
            floor_structures,
            floor_structures_paths,
            floor_mouths_paths,
        );

        (String::from(floor), layer)
    });

    let building = HashMap::from_par_iter(building_raw);

    println!("[INFO] Camp Nou loaded [{:?}]", start.elapsed());

    // Generate world
    let w = World {
        step: 0,
        agent_count: 0,
        arrivals: load_arrivals(),
        gates: load_gates(),
        gates_to_mouths: World::gates_mouths(&building, &load_gates()),
        gates_to_stairs: World::gates_stairs(&building, &load_gates()),
        building_conexions: World::connect_structures(&building),
        building,
    };

    println!("[INFO] Environment ready [{:?}]", start.elapsed());

    w
}

pub fn bincode_load(path: String) -> World {
    println!("[INFO] Loading bincode version...");

    let start = Instant::now();

    let file = BufReader::new(File::open(path).unwrap());

    let w = deserialize_from(file).unwrap();

    println!("[INFO] Elapsed time: {:?}", start.elapsed());

    w
}
