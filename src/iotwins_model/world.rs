use std::{collections::HashMap, fs::File};

use indicatif::{ParallelProgressIterator, ProgressBar, ProgressStyle};
use rayon::prelude::*;

use crate::{
    config::configuration::Parameters,
    engine::{path_finding::a_star, routes::ConcurrentHashMap},
    iotwins_model::{
        config::arrival::{self, Arrival},
        stadium,
        structures::Jump,
    },
};

pub struct World {
    pub building: HashMap<String, stadium::Floor>,
    step: u64,
    pub gates: HashMap<String, Vec<usize>>,
    pub mouths: HashMap<String, HashMap<i16, Vec<usize>>>,
    pub arrivals: HashMap<i32, Vec<Arrival>>,
    pub stairs: HashMap<String, Vec<Jump>>,
}

impl World {
    pub fn default_paths(&self) {
        println!("[INFO] Finding routes");

        self.stairs
                .iter()
                .for_each(|(level, stairs)| {

                    // Empty HashMap for each layer
                    let routes = ConcurrentHashMap::new();
                    
                    // Cloned stairs
                    let mut layer_stairs = stairs.clone();

                    // Ground thruth
                    let floor_gt = &self
                        .building
                        .get(level)
                        .expect("[ERROR] No such layer")
                        .ground_truth;

                    // // Stairs position on layer
                    // let mut stairs_position: Vec<Vec<usize>> = stairs
                    //     .iter()
                    //     .map(|jump| jump.location.to_vec())
                    //     .collect();
                    
                    // TODO: Change this to the new influence area 

                    // Converts vector to FIFO queue, this way we get rid of recomputating paths
                    while let Some(stair) = layer_stairs.pop() {

                        // Matrix location for all other structures in `stair` influence area
                        let destinations: Vec<usize> = stair.influence_area(stairs, 300.0).iter().flat_map(|j| j.location.to_vec()).collect();

                        stair.location.iter().for_each(|position| {

                            // Progress bar
                            let progress_bar =
                                ProgressBar::new(destinations.len().try_into().unwrap());

                            progress_bar.set_message(format!("{} - {}", level, layer_stairs.len()));

                            progress_bar.set_style(
                                ProgressStyle::default_spinner()
                                    .template(
                                        "{spinner} {msg} {elapsed_precise} {bar:10} {pos}/{len} [{percent}% - {eta_precise}]",
                                    ).progress_chars("#>#-"));
                            // End of progress bar

                            // Multiple search
                            destinations
                                .par_iter()
                                .progress_with(progress_bar)
                                .for_each(|destination| {
                                    let path = a_star(
                                        floor_gt,
                                        *position,
                                        *destination
                                    );

                                    if !path.is_empty() {
                                        routes.insert(level.to_string(), *position, *destination, path);
                                    }
                                }
                            );
                        });
                    }

                    let std_routes = routes.convert_concurrent();

                    let file_path = format!("resources/627/paths/{level}.json");

                    let file = File::create(file_path).expect("[ERROR] No write permissions");

                    serde_json::to_writer(file, &std_routes).expect("[ERROR] No file");
                }
            );

        //TODO: Gates to mouths
    }
}

// Generate a unique HashMap with the whole simulation with index for checkpointing and agents
pub fn create_world(configuration: Parameters) -> World {
    let floors = configuration.topology.layers();
    let mut building: HashMap<String, stadium::Floor> = HashMap::new();

    println!("[INFO] Creating world");

    floors.into_iter().for_each(|(floor, path)| {
        let layer = stadium::Floor::load_floor(
            floor.to_string(),
            path.to_string(),
            configuration.get_world_size(),
        );

        building.insert(floor.to_string(), layer);
    });

    World {
        stairs: Jump::find_locations(&building),
        step: 0,
        gates: stadium::load_gates(configuration.venue_tags.gates_info),
        mouths: stadium::load_mouths(configuration.venue_tags.mouths_info),
        arrivals: arrival::load_arrivals(configuration.venue_tags.arrivals_info_csv),
        building,
    }
}
