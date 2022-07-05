use std::{
    collections::{BinaryHeap, HashMap, HashSet, VecDeque},
    fs::{self, File},
    io::{BufReader, BufWriter},
    time::Instant,
};

use bincode::{deserialize_from, serialize_into};
use indicatif::{ParallelProgressIterator, ProgressBar, ProgressIterator, ProgressStyle};
use rand::{distributions::Uniform, prelude::SliceRandom};
use rayon::prelude::*;
use serde::{Deserialize, Serialize};

use crate::{
    config::configuration::Parameters,
    engine::{
        path_finding::a_star,
        saving::{self, PathSegment},
    },
    iotwins_model::{
        agent::Agent,
        arrivals::{load_arrivals, Arrival},
        routes::{find_route, Route},
        stadium::{self},
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
    pub gates_buffer: HashMap<Gate, VecDeque<Arrival>>,
    pub gates_to_stairs: HashMap<Gate, HashSet<Route>>,
    pub gates_to_mouths: HashMap<Gate, HashMap<u16, Route>>,
    pub agent_path: HashMap<usize, BinaryHeap<PathSegment>>,
    pub agent_target: HashMap<usize, u16>,
}

impl World {
    // // Returns closest structure with exit to the desired layer
    // pub fn get_closest_conexion(
    //     &self,
    //     origen: &Structure,
    //     floor: &String,
    //     destination_floor: &String,
    // ) -> Option<Structure> {
    //     let level = self.building_conexions.get(floor).unwrap();

    //     // Get only stairs connecting to destination_floor
    //     let search_space: Vec<Structure> = level
    //         .iter()
    //         .filter_map(
    //             |(structure, conexions)| match conexions.contains_key(destination_floor) {
    //                 true => Some(structure.to_owned()),
    //                 false => None,
    //             },
    //         )
    //         .collect();

    //     // Return closest structure
    //     origen.get_closest_structure(&search_space)
    // }

    pub fn evolve(&mut self, interest: Uniform<f64>) -> usize {
        // 0.3s per step, 200 steps make a minute. Agent arrival only executed once per minute
        if self.step % 200 == 0 {
            self.load_arrival(); // Agent arrivals
        }

        // Gates queues are freeded progresively (one arrival per 15 secs)
        if self.step % 25 == 0 {
            self.gate_entrance(interest);
        }

        // All floors are executed at the same time
        let swapping_agents: HashMap<String, HashMap<Agent, Vec<usize>>> = HashMap::from_par_iter(
            self.building
                .par_iter_mut()
                .map(|(layer, floor)| (layer.to_string(), floor.evolve_floor(interest))),
        );

        // Store local path
        self.save_local_paths(&swapping_agents);

        // Move agents into buffers THIS DO NOT WORK
        let swapped = self.swap_agents(HashMap::from_iter(swapping_agents.into_iter().map(
            |(leaving_layer, agents)| {
                (
                    leaving_layer,
                    agents.keys().into_iter().map(|ag| ag.to_owned()).collect(),
                )
            },
        )));

        self.step += 1;
        swapped
    }

    // Arrivals are queued up for each gate
    fn load_arrival(&mut self) -> i32 {
        let time = self.get_time();
        let mut total = 0;

        if let Some(arrivals) = self.arrivals.get(&time) {
            arrivals.iter().for_each(|arrival| {
                // Remove untracked gates (VIP)
                if let Some(gate) = self.gates.get(&Gate {
                    name: arrival.gate.to_string(),
                    ..Default::default()
                }) {
                    self.gates_buffer
                        .get_mut(gate)
                        .unwrap()
                        .push_back(arrival.to_owned());

                    total += 1;
                } else {
                    // Delete arrival
                    println!("[INFO] Wrong arrival {} agents", arrival.agents);
                }
            });
        }

        total
    }

    // Arrivals are converted into agents
    fn gate_entrance(&mut self, interest: Uniform<f64>) -> usize {
        let mut total_inserted = 0;

        self.gates_buffer.iter_mut().for_each(|(gate, queue)| {
            if let Some(arrival) = queue.pop_front() {
                match arrival.gate_layer() == arrival.mouth_layer() {
                    true => {
                        // Agent does not leave floor
                        let floor = self.building.get_mut(&arrival.gate_layer()).unwrap();

                        let gate_routes = self.gates_to_mouths.get(gate).unwrap();

                        if let Some(route) = gate_routes.get(&arrival.mouth) {
                            let target = floor.mouths.get(&arrival.mouth).unwrap();

                            let agents = arrival.generate_agents(
                                target.to_owned(),
                                self.agent_count,
                                interest,
                            );

                            self.agent_count += agents.len();
                            total_inserted += floor.insert_agents(&agents, route.to_owned());
                        } else {
                            // No precomputed path, another try is done
                            let origin = gate
                                .structure
                                .location
                                .choose(&mut rand::thread_rng())
                                .unwrap();

                            let target = floor.mouths.get(&arrival.mouth).unwrap();

                            if let Some(path) = a_star(
                                &floor.ground_truth,
                                *origin,
                                *target.location.choose(&mut rand::thread_rng()).unwrap(),
                            ) {
                                let agents = arrival.generate_agents(
                                    target.to_owned(),
                                    self.agent_count,
                                    interest,
                                );

                                self.agent_count += agents.len();
                                total_inserted += agents.len();

                                let agents_paths =
                                    agents.iter().map(|agent| (agent.id, path.to_vec()));

                                floor.agents_paths.extend(agents_paths);
                                floor.agents.extend(agents.to_vec());

                                total_inserted += agents.len();
                            }
                        }
                    }
                    false => {
                        if let Some(floor) = self.building.get(&arrival.mouth_layer()) {
                            let mouth_stairs: HashSet<Structure> =
                                HashSet::from_iter(
                                    floor.mouths_paths.get(&arrival.mouth).into_iter().flat_map(
                                        |route| route.iter().map(|r| r.origin.to_owned()),
                                    ),
                                );

                            // Gate & mouth in different floor

                            let floor = self.building.get_mut(&arrival.gate_layer()).unwrap();

                            let level = self.building_conexions.get(&arrival.gate_layer()).unwrap();
                            let gate_routes = self.gates_to_stairs.get(gate).unwrap();

                            // Get only stairs connecting to destination_floor and access to destination mouth
                            let search_space: Vec<Structure> = level
                                .iter()
                                .filter_map(|(structure, conexions)| {
                                    match conexions.get(&arrival.mouth_layer()) {
                                        Some(s) => match mouth_stairs.contains(s) {
                                            true => Some(structure.to_owned()),
                                            false => None,
                                        },
                                        None => None,
                                    }
                                })
                                .collect();

                            if let Some(target) =
                                gate.structure.get_closest_structure(&search_space)
                            {
                                match gate_routes.get(&Route {
                                    origin: gate.to_owned().structure,
                                    destination: target.to_owned(),
                                    ..Default::default()
                                }) {
                                    Some(route) => {
                                        let agents = arrival.generate_agents(
                                            target,
                                            self.agent_count,
                                            interest,
                                        );

                                        self.agent_count += agents.len();
                                        total_inserted += agents.len();

                                        floor.insert_agents(&agents, route.to_owned());
                                    }
                                    None => {}
                                }
                            }
                        }
                    }
                }
            }
        });

        total_inserted
    }

    fn swap_agents(&mut self, swap: HashMap<String, Vec<Agent>>) -> usize {
        let mut total_swaped = 0;

        swap.into_iter().for_each(|(leaving_layer, agents)| {
            agents.into_iter().for_each(|mut agent| {
                if let Some(up_stair_cons) = self
                    .building_conexions
                    .get(&leaving_layer)
                    .unwrap()
                    .get(&agent.target)
                {
                    let destination_structure =
                        up_stair_cons.get(&agent.destination_layer).unwrap();

                    let destination_floor =
                        self.building.get_mut(&agent.destination_layer).unwrap();

                    agent.steps = 0;
                    agent.target = destination_structure.to_owned();

                    destination_floor.swap_buffer(&mut agent.to_owned(), destination_structure);
                    total_swaped += 1;
                } else {
                    // Agent arrived at destination: end of path at destination layer
                }
            });
        });

        total_swaped
    }

    fn save_local_paths(&mut self, paths: &HashMap<String, HashMap<Agent, Vec<usize>>>) {
        paths.iter().for_each(|(layer, agents)| {
            agents.iter().for_each(|(agent, local_path)| {
                let global_path = self
                    .agent_path
                    .entry(agent.id)
                    .or_insert_with(BinaryHeap::new);

                global_path.push(PathSegment::new(
                    agent,
                    local_path.to_vec(),
                    layer,
                    self.step,
                ));

                // Store agent final destination (mouth) (is done once)
                self.agent_target
                    .entry(agent.id)
                    .or_insert(agent.destination);
            });
        });
    }

    fn get_time(&mut self) -> i32 {
        (self.step as i32 / 100) - 150
    }

    // Creates a CSV for visualization purposes
    pub fn generate_save(&mut self) {
        File::create("resources/paths.csv").unwrap();

        let file = BufWriter::new(
            fs::OpenOptions::new()
                .write(true)
                .append(true)
                .open("resources/paths.csv")
                .unwrap(),
        );

        let mut writter = csv::Writer::from_writer(file);

        writter
            .write_record(&["agent_id", "x", "y", "layer", "step", "target_mouth"])
            .unwrap();

        self.agent_path.iter_mut().for_each(|(agent_id, path)| {
            let target_layer = self.agent_target.get(agent_id).unwrap();

            saving::generate_path(*agent_id, path, target_layer)
                .into_iter()
                .for_each(|record| writter.write_record(record).unwrap());
        });
    }

    pub fn save_structures(&self) {
        println!("[INFO] Saving structures...");
        let mut data = HashMap::new();

        self.building.iter().for_each(|(layer, floor)| {
            data.insert(layer, floor.structures.to_owned());
        });

        let file1 = BufWriter::new(File::create("resources/stairs.json").unwrap()); // All structures by layer
        serde_json::to_writer_pretty(file1, &data).expect("");
    }

    pub fn save_layer_paths(&self) {
        println!("[INFO] Saving paths...");

        // All routes in layer
        for (layer, floor) in &self.building {
            let file1 = File::create(format!("resources/stairs_paths/{layer}.json")).unwrap();

            let data1: Vec<Route> = floor.structures_paths.iter().cloned().collect();

            serde_json::to_writer_pretty(file1, &data1).expect("");

            let file2 = File::create(format!("resources/mouths_paths/{layer}.json")).unwrap();

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

        progress_bar.enable_steady_tick(1000);

        progress_bar.set_style(
            ProgressStyle::default_spinner()
                .template("{msg} {bar:10} {pos}/{len} {eta_precise}")
                .progress_chars("#>#-"),
        );
        // End of progress bar

        let routes = gates.iter().progress_with(progress_bar).map(|gate| {
            let floor = building.get(&gate.floor).unwrap();
            let up_stairs = floor.structures.get(&11).unwrap();

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

        progress_bar.enable_steady_tick(1000);

        progress_bar.set_style(
            ProgressStyle::default_spinner()
                .template("{msg} {bar:10} {pos}/{len} {eta_precise}")
                .progress_chars("#>#-"),
        );
        // End of progress bar

        let routes = gates.iter().progress_with(progress_bar).map(|gate| {
            let floor = building.get(&gate.floor).expect("");

            let gate_routes = floor.mouths.par_iter().filter_map(|(id, mouth)| {
                find_route(&floor.ground_truth, &gate.structure, mouth).map(|route| (*id, route))
            });

            (gate.to_owned(), HashMap::from_par_iter(gate_routes))
        });

        HashMap::from_iter(routes)
    }

    // For each structure in floor gets their destination (Links stairs between layers). Generates proper global structure between them all
    fn connect_structures(
        building: &HashMap<String, stadium::Floor>,
    ) -> HashMap<String, HashMap<Structure, HashMap<String, Structure>>> {
        let conexions = building.iter().map(|(layer, floor)| {
            let up_structures = floor.structures.get(&11).unwrap();

            // Progress bar
            let progress_bar = ProgressBar::new(up_structures.len().try_into().unwrap());

            progress_bar.set_message(format!("{layer} - Up stairs"));

            progress_bar.enable_steady_tick(1000);

            progress_bar.set_style(
                ProgressStyle::default_spinner()
                    .template("{spinner} {msg} {bar:10} {pos}/{len} {eta_precise}")
                    .progress_chars("#>#-"),
            );

            let layer_stairs =
                up_structures
                    .par_iter()
                    .progress_with(progress_bar)
                    .map(|structure| {
                        let structure_exits = building
                            .par_iter()
                            .filter(|(arriving_layer, _)| arriving_layer.to_string() != *layer) // Remove current floor
                            .filter_map(|(arriving_layer, arriving_floor)| {
                                let search_space = arriving_floor.structures.get(&10).unwrap();

                                match search_space.is_empty() {
                                    true => None, // No aviable conexion
                                    false => structure
                                        .get_closest_structure(&Vec::from_iter(
                                            search_space.to_owned(),
                                        ))
                                        .map(|structure| (arriving_layer.to_string(), structure)),
                                }
                            });

                        (
                            structure.to_owned(),
                            HashMap::from_par_iter(structure_exits),
                        )
                    });

            (layer.to_string(), HashMap::from_par_iter(layer_stairs))
        });
        HashMap::from_iter(conexions)
    }
}

// Generate a unique HashMap with the whole simulation with index for checkpointing and agents
pub fn create_world(configuration: Parameters) -> World {
    let floors = configuration.topology.layers();

    println!("[INFO] Creating world");
    let start = Instant::now();

    let building = HashMap::from_iter(floors.into_iter().map(|(floor, path)| {
        (
            floor.to_string(),
            stadium::Floor::create_floor(path, floor.to_string()),
        )
    }));

    println!("[INFO] Building created");

    let w = World {
        step: 0,
        agent_count: 0,
        building_conexions: World::connect_structures(&building),
        gates: load_gates(),
        gates_buffer: HashMap::from_iter(
            load_gates()
                .iter()
                .map(|gate| (gate.to_owned(), VecDeque::new())),
        ),
        gates_to_stairs: World::gates_stairs(&building, &load_gates()),
        gates_to_mouths: World::gates_mouths(&building, &load_gates()),
        arrivals: load_arrivals(),
        building,
        agent_path: HashMap::new(),
        agent_target: HashMap::new(),
    };

    println!("[INFO] Environment created [{:?}]", start.elapsed());

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
