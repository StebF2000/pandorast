use crate::{
    engine::{matrix::Matrix, path_finding},
    iotwins_model::{
        agent::Agent,
        routes::{find_route, Route},
        structures::{generate_structures, load_mouths, Structure},
    },
};
use dashmap::DashMap;
use indicatif::{ParallelProgressIterator, ProgressBar, ProgressStyle};
use rand::{distributions::Uniform, prelude::SliceRandom};
use rayon::prelude::*;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet, VecDeque};

#[derive(Clone, Default, Serialize, Deserialize)]
pub struct Floor {
    pub ground_truth: Matrix<u8>,
    pub structures: HashMap<u8, HashSet<Structure>>, // Mapping Position -> matrix by type of structure. MAYBE NOT NEEDED?
    pub structures_buffer: HashMap<Structure, VecDeque<Agent>>,
    pub structures_paths: HashSet<Route>,
    pub mouths: HashMap<u16, Structure>, // Agent destinations
    pub mouths_paths: HashMap<u16, HashSet<Route>>, // From down-stairs -> mouths (grandstands)
    pub agents: Vec<Agent>,              // All agents in floor
    pub agents_paths: DashMap<usize, Vec<usize>>, // Path to follow by every agent in layer (Only own agent modifies this, analytics)
}

impl Floor {
    pub fn create_floor(path: String, name: String) -> Floor {
        let ground_truth = Floor::ground_truth(&Matrix::load_layer(&path));
        let structures = generate_structures(&ground_truth);
        let mouths = load_mouths(&name);

        Floor {
            structures_paths: Floor::stairs_paths(&ground_truth, &structures, &name),
            mouths_paths: Floor::mouth_paths(&ground_truth, &structures, &mouths, &name),
            mouths,
            // Down-stairs get their buffer for arriving agents from other layers
            structures_buffer: HashMap::from_iter(
                structures
                    .get(&10)
                    .unwrap()
                    .iter()
                    .map(|structure| (structure.to_owned(), VecDeque::new())),
            ),
            structures,
            ground_truth,
            ..Default::default()
        }
    }

    pub fn insert_agents(
        &mut self,
        agents: &[Agent],
        route: Route, // Agent arrival on gate
    ) -> usize {
        let agents_paths = agents.iter().map(|agent| (agent.id, route.get_path()));

        self.agents_paths.extend(agents_paths);
        self.agents.extend(agents.to_vec());

        agents.len()
    }

    pub fn swap_buffer(&mut self, agent: &mut Agent, stair: &Structure) {
        match self.structures_buffer.get_mut(stair) {
            Some(buffer) => buffer.push_back(std::mem::take(agent)),
            None => {
                // Do nothing, agent will desappear
                println!("[WARR] Agent {} deleted", agent.id);
            }
        }
    }

    fn insert_buffered_agents(&mut self) {
        self.structures_buffer
            .iter_mut()
            .for_each(|(stair, buffer)| {
                // If there is any agent on hold
                if let Some(mut agent) = buffer.pop_front() {
                    let destination = self.mouths.get(&agent.destination).unwrap();
                    let routes = self.mouths_paths.get(&agent.destination).unwrap();

                    if let Some(route) = routes.get(&Route {
                        origin: stair.to_owned(),
                        destination: destination.to_owned(),
                        ..Default::default()
                    }) {
                        let path = route.get_path();
                        agent.next_step = path[1];
                        agent.steps = 1;

                        self.agents_paths.insert(agent.id, path);
                        self.agents.push(agent.to_owned());
                    }
                }
                // If there is no route agent is lost
            });
    }

    pub fn evolve_floor(&mut self, interest: Uniform<f64>) -> HashMap<Agent, Vec<usize>> {
        // Remove end of path agents
        let leaving: HashMap<Agent, Vec<usize>> =
            HashMap::from_iter(self.agents.drain_filter(|ag| ag.next_step == 0).map(|ag| {
                let (_, path) = self.agents_paths.remove(&ag.id).unwrap();
                (ag, path)
            }));

        // Add agents from stairs
        self.insert_buffered_agents();

        // Evolve non-conflicting ones in parallel
        let no_conflict = self.conficts();

        self.agents
            .par_iter_mut()
            .filter(|ag| no_conflict.contains(&ag.next_step))
            .for_each(|ag| {
                let ag_path = ag.action(
                    interest,
                    &mut self.agents_paths.get_mut(&ag.id).unwrap(),
                    &self.ground_truth,
                );
            });

        // Conflicting agents
        self.agents
            .iter_mut()
            .filter(|ag| !no_conflict.contains(&ag.next_step))
            .for_each(|ag| {
                let movements: Vec<usize> =
                    path_finding::movements(ag.next_step, &self.ground_truth)
                        .into_iter()
                        .filter(|idx| !no_conflict.contains(idx))
                        .collect();

                let mut ag_path = self.agents_paths.get_mut(&ag.id).unwrap();

                if !movements.is_empty() {
                    // Agent will move to other place and wander arround
                    let mut wander_path =
                        vec![*movements.choose(&mut rand::thread_rng()).unwrap(); 10];

                    ag.next_wandering = 15;

                    (1_usize..10).into_iter().for_each(|i| {
                        wander_path[i] =
                            *path_finding::movements(wander_path[i - 1], &self.ground_truth)
                                .choose(&mut rand::thread_rng())
                                .unwrap();
                    });

                    let join_position = ag.steps + ((ag_path.len() - ag.steps) / 2);

                    match path_finding::a_star(&self.ground_truth, wander_path[9], join_position) {
                        Some(mut join_path) => {
                            // There is a possible path to rejoin
                            wander_path.extend(join_path.drain(1..));

                            // Path is updated with wandering
                            ag_path.splice(ag.steps..join_position, wander_path);
                        }
                        None => {
                            // Path is reversed
                            wander_path.extend(wander_path.to_owned().drain(1..).rev());

                            ag_path.splice(ag.steps..ag.steps, wander_path);
                        }
                    }
                } else {
                    // No positions, stays in place
                    let curr_pos = ag_path[ag.steps];
                    ag.steps += 1;
                    ag_path.insert(ag.steps, curr_pos);
                    ag.next_wandering = 1; // This prevents from natural action.
                }

                ag.action(interest, &mut ag_path, &self.ground_truth);
            });

        leaving
    }

    // Independent positions are returned
    fn conficts(&mut self) -> HashSet<usize> {
        self.agents
            .iter()
            .map(|ag| ag.next_step)
            .collect::<HashSet<usize>>()
    }

    fn stairs_paths(
        gt: &Matrix<u8>,
        structures: &HashMap<u8, HashSet<Structure>>,
        layer: &str,
    ) -> HashSet<Route> {
        let down = structures.get(&10).expect("");
        let up = structures.get(&11).expect("");

        // Progress bar
        let progress_bar = ProgressBar::new(down.len().try_into().unwrap());

        progress_bar.set_message(format!("{layer} - Map jumps"));

        progress_bar.enable_steady_tick(1000);

        progress_bar.set_style(
            ProgressStyle::default_spinner()
                .template("{msg} {bar:10} {pos}/{len} {eta_precise}")
                .progress_chars("#>#-"),
        );
        // End of progress bar

        let stairs_paths = down
            .par_iter()
            .progress_with(progress_bar)
            .flat_map(|p1| up.par_iter().filter_map(|p2| find_route(gt, p1, p2)));

        HashSet::from_par_iter(stairs_paths)
    }

    // For each destination mouth, a set of possible routes from up-stairs
    fn mouth_paths(
        gt: &Matrix<u8>,
        structures: &HashMap<u8, HashSet<Structure>>,
        mouths: &HashMap<u16, Structure>,
        layer: &str,
    ) -> HashMap<u16, HashSet<Route>> {
        // Down stairs are the only positions from where you can go to the grandstands, agents will arrive at down-stairs
        let down = structures.get(&10).expect("");

        // Progress bar
        let progress_bar = ProgressBar::new(down.len().try_into().unwrap());

        progress_bar.set_message(format!("{layer} - Mouths paths"));

        progress_bar.enable_steady_tick(1000);

        progress_bar.set_style(
            ProgressStyle::default_spinner()
                .template("{msg} {bar:10} {pos}/{len} {eta_precise}")
                .progress_chars("#>#-"),
        );
        // End of progress bar

        let mouths_routes = mouths
            .par_iter()
            .progress_with(progress_bar)
            .map(|(id, mouth)| {
                let mouth_routes = down
                    .par_iter()
                    .filter_map(|stair| find_route(gt, stair, mouth));

                (*id, HashSet::from_par_iter(mouth_routes))
            });

        // mouths_paths
        HashMap::from_par_iter(mouths_routes)
    }

    pub fn ground_truth(blueprint: &Matrix<u8>) -> Matrix<u8> {
        Matrix::ground_thruth(
            blueprint,
            HashMap::from([
                // Walls
                (84, 1),  // Obstacles
                (202, 1), // No transit
                (159, 1), // Kitchen
                (7, 1),   // Technical staff
                (241, 1), // Lockers
                (103, 1), // SAT
                (131, 1), // Men restroom
                (118, 1), // Women restroom
                (164, 1), // Restroom
                (122, 1), // Bar
                // Map jumps
                (109, 10), // Down stair
                (232, 11), // Up stair
                (112, 20), // Down ramp
                (189, 21), // Up ramp
                (182, 3),  // Elevator
            ]),
        )
    }
}
