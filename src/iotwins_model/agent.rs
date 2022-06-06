use std::hash::{Hash, Hasher};

use rand::{distributions::Uniform, prelude::Distribution};
use serde::{Deserialize, Serialize};

use crate::iotwins_model::structures::Structure;

#[derive(Clone, Default, Serialize, Deserialize, Debug)]
pub struct Agent {
    pub id: usize,
    pub destination: u16,  // Final mouth
    pub target: Structure, // Current target
    interest: f32,
    steps: u16,
}

impl PartialEq for Agent {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

impl Eq for Agent {}

impl Hash for Agent {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.id.hash(state);
    }
}

impl Agent {
    // Does not assign inmediate destination, only final target
    pub fn new(
        id: usize,
        target: Structure,
        destination: u16,
        between: Uniform<f32>,
    ) -> Agent {
        let mut rng = rand::thread_rng();

        Agent {
            id,
            target: target.clone(),
            destination,
            interest: between.sample(&mut rng),
            ..Default::default()
        }
    }

    pub fn action(&mut self) {
        todo!()
    }
}
