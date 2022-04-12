pub mod simulation {
    use std::{collections::HashMap, fs::File};

    use indicatif::{ParallelProgressIterator, ProgressBar, ProgressStyle};
    use rayon::iter::{IntoParallelRefIterator, ParallelIterator};

    use crate::{
        config::configuration::Parameters,
        engine::{path_finding::a_star, routes::ConcurrentHashMap},
        iotwins_model::{
            self,
            config::arrival::{self, Arrival},
            stadium::{self, environment::Floor},
            structures::{self, tagging::Jump},
        },
    };

    pub struct World {
        pub building: HashMap<String, Floor>,
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

                        let routes = ConcurrentHashMap::new();

                        // Ground thruth
                        let floor_gt = &self
                            .building
                            .get(level)
                            .expect("[ERROR] No such layer")
                            .ground_truth;

                        // Stairs position on layer (Constantly less than 15 elements)
                        let mut stairs_position: Vec<Vec<usize>> = stairs
                            .iter()
                            .map(|jump| jump.location.to_vec())
                            .collect();
                        // Converts vector to FIFO queue, this way we get rid of recomputating paths
                        while let Some(stair) = stairs_position.pop() {

                            stair.iter().for_each(|position| {
                                let destinations: Vec<usize> =
                                    stairs_position.iter().flatten().copied().collect();

                                // Progress bar
                                let progress_bar =
                                    ProgressBar::new(destinations.len().try_into().unwrap());

                                progress_bar.set_message(format!("{} - {}", level, stairs_position.len()));

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
        let mut building: HashMap<String, Floor> = HashMap::new();

        println!("[INFO] Creating world");

        floors.into_iter().for_each(|(floor, path)| {
            let layer = Floor::load_floor(
                floor.to_string(),
                path.to_string(),
                configuration.get_world_size(),
            );

            building.insert(floor.to_string(), layer);
        });

        World {
            stairs: structures::tagging::Jump::find_locations(&building),
            step: 0,
            gates: stadium::environment::load_gates(configuration.venue_tags.gates_info),
            mouths: stadium::environment::load_mouths(configuration.venue_tags.mouths_info),
            arrivals: arrival::load_arrivals(configuration.venue_tags.arrivals_info_csv),
            building,
        }
    }
}
