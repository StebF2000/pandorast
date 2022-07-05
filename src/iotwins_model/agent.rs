use std::hash::{Hash, Hasher};

use rand::{
    distributions::Uniform,
    prelude::{Distribution, SliceRandom},
};
use serde::{Deserialize, Serialize};

use crate::{
    engine::{matrix::Matrix, path_finding},
    iotwins_model::structures::Structure,
};

#[derive(Clone, Default, Serialize, Deserialize, Debug)]
pub struct Agent {
    pub id: usize,
    pub destination: u16, // Final mouth
    pub destination_layer: String,
    pub target: Structure, // Current target
    interest: f64,
    pub steps: usize,
    pub next_step: usize,
    pub next_wandering: usize,
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
        destination_layer: String,
        between: Uniform<f64>,
    ) -> Agent {
        let mut rng = rand::thread_rng();

        Agent {
            id,
            target,
            destination,
            destination_layer,
            interest: between.sample(&mut rng),
            ..Default::default()
        }
    }

    pub fn action(&mut self, interest: Uniform<f64>, path: &mut Vec<usize>, gt: &Matrix<u8>) {
        // Interest decrement by 3%
        if self.steps % 100 == 0 {
            self.interest *= 0.97
        }

        if self.next_wandering != 0 {
            // Avoid multiple wanderings together as wander route is already calculated
            self.next_wandering -= 1;
        } else {
            let choice = interest.sample(&mut rand::thread_rng());

            // Choice > interest -> regular walk everything else here:
            if choice < self.interest * 0.20 {
                // Stop
                self.steps += 1;
                path.insert(self.steps, path[self.steps]);
            } else if choice < self.interest {
                // Wandering
                self.next_wandering = 30;

                let mut wander_path = vec![path[self.steps]; 15];

                (1_usize..15).into_iter().for_each(|i| {
                    wander_path[i] = *path_finding::movements(wander_path[i - 1], gt)
                        .choose(&mut rand::thread_rng())
                        .unwrap();
                });

                let join_position = self.steps + ((path.len() - self.steps) / 4);

                match path_finding::a_star(gt, wander_path[9], path[join_position]) {
                    Some(mut join_path) => {
                        wander_path.extend(join_path.drain(1..));

                        // Path is updated with wandering
                        path.splice(self.steps..join_position, wander_path);
                    }
                    None => {
                        // If there is no aviable path, agent will backtrack to their previous position
                        let mut join_path: Vec<usize> = wander_path.iter().copied().rev().collect();
                        wander_path.extend(join_path.drain(1..));

                        path.splice(self.steps..self.steps, wander_path);
                    }
                }
            }
        }

        // Once the path has been updated, agent moves
        self.steps += 1;

        // Agent announces its next movement
        if path.len() > self.steps {
            self.next_step = 0;
        } else {
            self.next_step = path[self.steps + 1];
        }
    }
}
