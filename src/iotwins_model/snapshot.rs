use std::{
    collections::{HashMap, VecDeque},
    fs::File,
    io::BufWriter,
};

use bincode::serialize_into;
use serde::{Deserialize, Serialize};

use crate::iotwins_model::agent::Agent;

#[derive(Serialize, Deserialize)]
pub struct Data {
    agents: Vec<Agent>,
    paths: HashMap<usize, VecDeque<usize>>,
}

impl Data {
    pub fn new(agents: Vec<Agent>, paths: HashMap<usize, VecDeque<usize>>) -> Data {
        Data { agents, paths }
    }
}

#[derive(Serialize, Deserialize)]
pub struct Snapshot {
    pub iter: u32,
    pub building: HashMap<String, Data>,
}

impl Snapshot {
    pub fn new(iter: usize, building: HashMap<String, Data>) -> Snapshot {
        todo!()
    }

    // Save snapshot for rollback
    pub fn write_snapshot(&self) {
        let path = format!("resources/snapshots/snapshot_{}", self.iter);

        let file = BufWriter::new(File::create(path).unwrap());

        serialize_into(file, &self).unwrap();
    }
}
