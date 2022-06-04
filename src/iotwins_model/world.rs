use std::{
    collections::{HashMap, HashSet},
    fs::{read_dir, File, ReadDir},
    io::{BufReader, BufWriter},
    time::Instant,
};

use bincode::serialize_into;
use indicatif::{ProgressBar, ProgressIterator, ProgressStyle};
use rand::distributions::Uniform;
use rayon::prelude::*;
use serde::{Deserialize, Serialize};

use crate::{
    config::configuration::Parameters,
    iotwins_model::{
        arrivals::{load_arrivals, Arrival},
        routes::{find_route, Route},
        stadium::{self, Floor},
        structures::{load_gates, Gate, Structure},
    },
};

#[derive(Serialize, Deserialize)]
pub struct World {
    pub building: HashMap<String, stadium::Floor>,
    pub building_conexions: HashMap<String, HashMap<Structure, HashMap<String, Structure>>>,
    step: u32,
    agent_count: usize,
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
        self.step += 1;

        let time = self.get_time(); // Return real

        // 1. Insert new agents into the simulation
        // 1.1 Check if agents should be inserted
        match self.arrivals.to_owned().get(&time) {
            // 1.2 Consume arrival
            Some(arrivals) => {
                arrivals.iter().for_each(|arrival| {
                    let n_agents = arrival.agents;

                    // This serves the purpose of looking for the right object already stored
                    match self.gates.get(&Gate {
                        name: arrival.gate.to_string(),
                        ..Default::default()
                    }) {
                        // Generate new agents
                        Some(origen) => {
                            match self.get_closest_conexion(
                                &origen.structure,
                                &origen.floor,
                                &arrival.mouth_layer(),
                            ) {
                                Some(target) => {
                                    let agents = arrival.generate_agents(
                                        &target,
                                        self.agent_count,
                                        interest,
                                    );
                                    // 1.3 Insert agents into simulation
                                    let floor =
                                        self.building.get_mut(&arrival.gate_layer()).unwrap();

                                    floor.insert_agents(agents, origen.structure.clone());
                                    println!("[INFO] MIN: {time} -> {n_agents} inserted");
                                }
                                None => {}
                            }
                        }
                        None => {}
                    }
                });
            }
            None => {}
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
        // Save building completely

        let mut file = BufWriter::new(File::create("IoTwins.bin").unwrap());

        serialize_into(&mut file, &self).unwrap();
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

    // Get real time from step (initial time is -90 minutes to match start)
    #[inline(always)]
    fn get_time(&self) -> i32 {
        (0.15 * self.step as f32) as i32 - 90_i32
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
        let layer = stadium::Floor::create_floor(path.to_string(), floor.to_string());

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
            path,
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
