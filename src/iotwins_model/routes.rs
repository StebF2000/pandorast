use rand::prelude::SliceRandom;
use serde::{Deserialize, Serialize};
use std::hash::{Hash, Hasher};

use rayon::prelude::*;

use crate::{
    engine::{matrix::Matrix, path_finding::a_star},
    iotwins_model::structures::Structure,
};
#[inline(always)]
pub fn find_route(gt: &Matrix<u8>, p1: &Structure, p2: &Structure) -> Option<Route> {
    let routes: Vec<Vec<usize>> = p1
        .location
        .par_iter()
        .flat_map(|p1| p2.location.par_iter().filter_map(|p2| a_star(gt, *p1, *p2)))
        .collect();

    // Checks if routes is empty for not creating such struct
    match routes
        .iter()
        .flatten()
        .copied()
        .into_iter()
        .next()
        .is_none()
    {
        true => None,
        false => Some(Route {
            origin: p1.to_owned(),
            destination: p2.to_owned(),
            paths: routes,
        }),
    }
}

#[derive(Clone, Eq, Serialize, Deserialize, Default)]
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
            || self.destination == other.origin && self.origin == other.destination
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
            origin: self.destination.to_owned(),
            destination: self.origin.to_owned(),
            paths: self
                .paths
                .iter()
                .map(|path| path.iter().copied().rev().collect())
                .collect(),
        }
    }
}
