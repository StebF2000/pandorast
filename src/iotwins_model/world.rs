use std::{
    collections::{HashMap, HashSet},
    fs::File,
};

use crate::{
    config::configuration::Parameters,
    iotwins_model::{
        routes::Route,
        stadium,
        stadium::arrivals::{load_arrivals, Arrival},
        structures::{load_gates, Structure},
    },
};

pub struct World {
    pub building: HashMap<String, stadium::Floor>,
    step: u64,
    resolution: u8,
    pub arrivals: HashMap<i32, Vec<Arrival>>,
    gates: HashMap<String, HashMap<String, Structure>>,
}

impl World {
    pub fn save_structures(&self) {
        let mut data = HashMap::new();

        for (layer, floor) in &self.building {
            data.insert(layer.to_string(), floor.structures.clone());
        }

        let file = File::create("resources/627/map_jumps.json").expect("");

        serde_json::to_writer(file, &data).expect("");
    }

    pub fn save_paths(&self) {
        let mut stairs_paths = HashMap::new();
        let mut mouths_paths = HashMap::new();

        for (layer, floor) in &self.building {
            stairs_paths.insert(layer.to_string(), floor.structures_paths.clone());
            mouths_paths.insert(layer.to_string(), floor.mouths_paths.clone());
        }

        let file1 = File::create("resources/627/stairs_paths.json").expect("");
        let file2 = File::create("resources/627/mouths_paths.json").expect("");

        serde_json::to_writer(file1, &stairs_paths).expect("");
        serde_json::to_writer(file2, &mouths_paths).expect("");
    }

    fn load_agents(&self) {
        let time = (self.step * self.resolution as u64) as i32;

        // let arrivals = self.arrivals
    }
}

// Generate a unique HashMap with the whole simulation with index for checkpointing and agents
pub fn create_world(configuration: Parameters) -> World {
    let floors = configuration.topology.layers();
    let mut building: HashMap<String, stadium::Floor> = HashMap::new();

    println!("[INFO] Creating world");

    floors.into_iter().for_each(|(floor, path)| {
        let layer = stadium::Floor::create_floor(path.to_string(), floor.to_string());

        building.insert(floor.to_string(), layer);
    });

    World {
        step: 0,
        resolution: 15, // Seconds
        arrivals: load_arrivals(),
        building,
        gates: load_gates(),
    }
}

pub fn load_world(
    structures_path: String,
    mouths_paths_path: String,
    structures_paths_path: String,
    configuration: Parameters,
) -> World {
    println!("[INFO] Loading world...");

    let mut building: HashMap<String, stadium::Floor> = HashMap::new();

    // Load pre-computed stuff (routes and structures)
    let structures_file = File::open(structures_path).expect("");
    let mouths_paths_file = File::open(mouths_paths_path).expect("");
    let structures_paths_file = File::open(structures_paths_path).expect("");

    let structures: HashMap<String, HashMap<u8, HashSet<Structure>>> =
        serde_json::from_reader(structures_file).expect("");

    let mouths_paths: HashMap<String, HashMap<u16, HashSet<Route>>> =
        serde_json::from_reader(mouths_paths_file).expect("");

    let structures_paths: HashMap<String, HashSet<Route>> =
        serde_json::from_reader(structures_paths_file).expect("");

    let floors = configuration.topology.layers();

    floors.into_iter().for_each(|(floor, path)| {
        let layer = stadium::Floor::load_floor(
            path.to_string(),
            floor.to_string(),
            mouths_paths.get(&floor.to_string()).expect("").clone(),
            structures_paths.get(&floor.to_string()).expect("").clone(),
        );

        building.insert(floor.to_string(), layer);
    });

    World {
        step: 0,
        resolution: 15, // Seconds
        arrivals: load_arrivals(),
        building,
        gates: load_gates(),
    }
}
