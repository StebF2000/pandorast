use rand::prelude::SliceRandom;
use serde::{Deserialize, Serialize};
use std::hash::{Hash, Hasher};

use rayon::prelude::*;

use crate::{
    engine::{matrix::Matrix, path_finding::a_star},
    iotwins_model::structures::Structure,
};
#[inline(always)]
pub fn find_route(gt: &Matrix<u8>, p1: &Structure, p2: &Structure) -> Route {
    let routes: Vec<Vec<usize>> = p1
        .location
        .par_iter()
        .flat_map_iter(|p1| p2.location.iter().map(|p2| a_star(gt, *p1, *p2)))
        .collect();

    Route {
        origin: p1.clone(),
        destination: p2.clone(),
        paths: routes,
    }
}

#[derive(Clone, Eq, Serialize, Deserialize)]
pub struct Route {
    pub origin: Structure,
    pub destination: Structure,
    pub paths: Vec<Vec<usize>>,
}

impl Hash for Route {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.origin.hash(state);
        self.destination.hash(state);
    }
}

impl PartialEq for Route {
    fn eq(&self, other: &Self) -> bool {
        self.origin == other.origin && self.destination == other.destination
    }
}

impl Route {
    // Returns a random path between the points for the agents -> O(1)
    pub fn get_path(&self) -> Vec<usize> {
        self.paths
            .choose(&mut rand::thread_rng())
            .expect("")
            .to_vec()
    }

    // Route is reversed
    pub fn inverse(&self) -> Self {
        Route {
            origin: self.destination.clone(),
            destination: self.origin.clone(),
            paths: self
                .paths
                .iter()
                .map(|path| path.iter().copied().rev().collect())
                .collect(),
        }
    }
}
