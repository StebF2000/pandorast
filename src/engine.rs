pub mod matrix {
    use image::io::Reader as ImageReader;
    use serde::{Deserialize, Serialize};
    use std::collections::HashMap;
    use std::hash::{Hash, Hasher};

    #[derive(Clone, Serialize, Deserialize)]
    pub struct Matrix<T> {
        pub data: Vec<T>,
        pub n_rows: usize, // For position purposes
    }

    impl Default for Matrix<u8> {
        fn default() -> Self {
            Matrix {
                data: vec![],
                n_rows: 0,
            }
        }
    }

    impl Matrix<u8> {
        // Given a HashMap, converts blueprint to a standard form for path finding algorithm to work in
        // Codification should be stablised for obstacles (value = 1)
        pub fn ground_thruth(layer: &Matrix<u8>, codification: HashMap<u8, u8>) -> Matrix<u8> {
            let gt: Vec<u8> = layer
                .data
                .iter()
                .map(|value| match codification.get(value) {
                    Some(value) => *value,
                    _ => 0_u8,
                })
                .collect();

            Matrix {
                data: gt,
                n_rows: layer.n_rows,
            }
        }

        pub fn contiguous(&self, position: usize) -> Vec<usize> {
            let limit = self.n_rows;

            let x = position / limit;
            let y = position % limit;

            match x {
                // First row
                0 => match y {
                    // First column
                    0 => Vec::from([position + 1, position + limit, position + limit + 1]),
                    // Last column
                    626 => Vec::from([position - 1, position + limit, position + limit - 1]),
                    _ => Vec::from([
                        position - 1,
                        position + 1,
                        position + limit - 1,
                        position + limit,
                        position + limit + 1,
                    ]),
                },
                // Last row
                626 => match y {
                    // First column
                    0 => Vec::from([position + 1, position - limit, position - limit + 1]),
                    // Last column
                    626 => Vec::from([position - 1, position - limit, position - limit - 1]),
                    _ => Vec::from([
                        position - 1,
                        position + 1,
                        position - limit - 1,
                        position - limit,
                        position - limit + 1,
                    ]),
                },
                _ => match y {
                    // First column
                    0 => Vec::from([
                        position + 1,
                        position - limit,
                        position - limit + 1,
                        position + limit,
                        position + limit + 1,
                    ]),
                    // Last column
                    626 => Vec::from([
                        position - 1,
                        position - limit,
                        position - limit - 1,
                        position + limit,
                        position + limit - 1,
                    ]),
                    _ => Vec::from([
                        position - 1,
                        position + 1,
                        position + (limit - 1),
                        position + limit,
                        position + (limit + 1),
                        position - (limit - 1),
                        position - limit,
                        position - (limit + 1),
                    ]),
                },
            }
        }

        // Load blueprint from resources
        pub fn load_layer(path: &str) -> Matrix<u8> {
            let image = ImageReader::open(path)
                .unwrap()
                .decode()
                .unwrap()
                .to_luma8();

            Matrix {
                data: image.as_raw().to_vec(),
                n_rows: image.height() as usize,
            }
        }
    }

    #[derive(Clone, Copy, Eq, Serialize, Deserialize, Default, Debug)]
    pub struct Position {
        pub x: usize,
        pub y: usize,
    }

    impl PartialEq for Position {
        fn eq(&self, other: &Self) -> bool {
            self.x == other.x && self.y == other.y
        }
    }

    impl Hash for Position {
        fn hash<H: Hasher>(&self, state: &mut H) {
            self.x.hash(state);
            self.y.hash(state);
        }
    }

    impl Position {
        pub fn new(idx: usize, grid_size: usize) -> Position {
            Position {
                x: idx % grid_size,
                y: idx / grid_size,
            }
        }

        pub fn middle_location(data: &[usize], grid_size: usize) -> Position {
            let p: Vec<Position> = data
                .iter()
                .map(|idx| Position::new(*idx, grid_size))
                .collect();
            Position::middle(&p)
        }

        pub fn middle(data: &[Position]) -> Position {
            let mut x = 0;
            let mut y = 0;

            data.iter().for_each(|p| {
                x += p.x;
                y += p.y;
            });

            Position {
                x: x / data.len(),
                y: y / data.len(),
            }
        }

        #[inline(always)]
        /// Euclidean distance without sqrt (a2 + b2)
        pub fn distance(&self, other: &Position) -> i32 {
            i32::pow(self.x as i32 - other.x as i32, 2)
                + i32::pow(self.y as i32 - other.y as i32, 2)
        }
    }
}

pub mod path_finding {
    use std::{
        cmp::Ordering,
        collections::{BinaryHeap, HashMap},
    };

    use crate::engine::{matrix::Matrix, path_finding};

    // A* algorithm form origin to destination, grid must be squared
    // origin and destination will be supposed to be in grid. Blueprint should be passed.
    pub fn a_star(gt: &Matrix<u8>, origin: usize, destination: usize) -> Option<Vec<usize>> {
        // Generates  heuristic field (parallel way) the closer you get, the lower is the penalization (like gradient descend)
        let cost_function: Vec<u64> = gt
            .data
            .iter()
            .enumerate()
            .map(|(i, _)| path_finding::heuristic(i, gt.n_rows))
            .collect();

        let mut dist: Vec<u64> = vec![u64::MAX; cost_function.len()]; // Initial cost (inf)
        let mut heap: BinaryHeap<State> = BinaryHeap::new();

        // Trackinkg State for each position in dist matrix
        // Better performance than cloning an array
        let mut list: HashMap<usize, State> = HashMap::new();

        // Cost at origin is None (0)
        dist[origin] = 0;

        let original_state = State {
            cost: 0_u64,      // Initial cost (None)
            position: origin, // Initial position
            previous: origin,
        };

        heap.push(original_state);
        list.insert(original_state.position, original_state);

        while let Some(current_state) = heap.pop() {
            if current_state.position == destination {
                let mut state = current_state;
                let mut path = Vec::new();

                // Backtracking. The state popped is the most recent version thus, the optimal one
                while let Some(p) = list.remove(&state.previous) {
                    path.push(p.position);
                    state = p;
                }

                // Reveresed the reversed path, getting the good one
                return Some(path.into_iter().rev().collect());
            }
            // If cost is over current best, current_state is discarded
            if current_state.cost > dist[current_state.position] {
                continue;
            }

            path_finding::movements(current_state.position, gt)
                .into_iter()
                .filter(|pos| pos < &(gt.n_rows * gt.n_rows))
                .for_each(|pos| {
                    // Cost towards next step (actual cost + step + heuristic)
                    let new_cost =
                        current_state.cost + 1_u64 + cost_function[current_state.position];

                    // Next aviable position (node) with current route
                    // If so, add it to the frontier and continue
                    if new_cost < dist[pos] {
                        // Update new cost
                        dist[pos] = new_cost;

                        // This way the last iteration is saved
                        list.insert(current_state.position, current_state);

                        let new_state = State {
                            cost: new_cost,
                            position: pos,
                            previous: current_state.position,
                        };

                        heap.push(new_state);
                    }
                });
        }

        // No route is found
        None
    }

    // Basic heuristic function, derivate-like where as you get closer to the target, the cost reduces
    // Diagonal values taking into account, should be adapted for obstacles
    // Chebyshev distance heuristic function
    #[inline(always)]
    fn heuristic(pos: usize, grid_size: usize) -> u64 {
        let row = pos / grid_size;
        let col = pos % grid_size;

        std::cmp::max(row, col) as u64
    }

    pub fn movements(position: usize, gt: &Matrix<u8>) -> Vec<usize> {
        let x = position / gt.n_rows;
        let y = position % gt.n_rows;

        match x {
            // First row
            0 => match y {
                // First column
                0 => Vec::from([position + 1, position + gt.n_rows])
                    .into_iter()
                    .filter(|pos| *pos < gt.n_rows * gt.n_rows && gt.data[*pos] != 1)
                    .collect(),
                // Last column
                626 => Vec::from([position - 1, position + gt.n_rows])
                    .into_iter()
                    .filter(|pos| *pos < gt.n_rows * gt.n_rows && gt.data[*pos] != 1)
                    .collect(),
                _ => Vec::from([position - 1, position + 1, position + gt.n_rows])
                    .into_iter()
                    .filter(|pos| *pos < gt.n_rows * gt.n_rows && gt.data[*pos] != 1)
                    .collect(),
            },
            // Last row
            626 => match y {
                // First column
                0 => Vec::from([position + 1, position - gt.n_rows])
                    .into_iter()
                    .filter(|pos| *pos < gt.n_rows * gt.n_rows && gt.data[*pos] != 1)
                    .collect(),
                // Last column
                626 => Vec::from([position - 1, position - gt.n_rows]),
                _ => Vec::from([position - 1, position + 1, position - gt.n_rows])
                    .into_iter()
                    .filter(|pos| *pos < gt.n_rows * gt.n_rows && gt.data[*pos] != 1)
                    .collect(),
            },
            _ => match y {
                // First column
                0 => Vec::from([position + 1, position - gt.n_rows, position + gt.n_rows])
                    .into_iter()
                    .filter(|pos| *pos < gt.n_rows * gt.n_rows && gt.data[*pos] != 1)
                    .collect(),
                // Last column
                626 => Vec::from([position - 1, position - gt.n_rows, position + gt.n_rows])
                    .into_iter()
                    .filter(|pos| *pos < gt.n_rows * gt.n_rows && gt.data[*pos] != 1)
                    .collect(),
                _ => Vec::from([
                    position - 1,
                    position + 1,
                    position + gt.n_rows,
                    position - gt.n_rows,
                ])
                .into_iter()
                .filter(|pos| *pos < gt.n_rows * gt.n_rows && gt.data[*pos] != 1)
                .collect(),
            },
        }
    }

    // Converting BinaryHeap from max-heap to min-heap (reversed comparation)
    // Includes  State, Ord and PartialOrd
    #[derive(Eq, PartialEq, Clone, Copy)]
    struct State {
        cost: u64,
        position: usize,
        previous: usize,
    }

    impl Ord for State {
        fn cmp(&self, other: &State) -> Ordering {
            other
                .cost
                .cmp(&self.cost)
                .then_with(|| self.position.cmp(&other.position))
        }
    }

    impl PartialOrd for State {
        fn partial_cmp(&self, other: &State) -> Option<Ordering> {
            Some(self.cmp(other))
        }
    }
}

pub mod saving {
    use std::{cmp::Ordering, collections::BinaryHeap};

    use serde::{Deserialize, Serialize};

    use crate::{engine::matrix::Position, iotwins_model::agent::Agent};

    #[derive(Serialize, Deserialize, Eq, PartialEq, Debug)]
    pub struct PathSegment {
        init_step: u32,
        path: Vec<usize>,
        agent_id: usize,
        layer: String,
    }

    // Stuff needed for correctly ordering bottom-to-top the segments
    impl Ord for PathSegment {
        fn cmp(&self, other: &Self) -> Ordering {
            other
                .layer
                .cmp(&self.layer)
                .then_with(|| self.layer.cmp(&other.layer))
        }
    }

    impl PartialOrd for PathSegment {
        fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
            Some(self.cmp(other))
        }
    }

    impl PathSegment {
        pub fn new(agent: &Agent, path: Vec<usize>, layer: &str, current_step: u32) -> PathSegment {
            PathSegment {
                init_step: current_step - agent.steps as u32,
                path,
                agent_id: agent.id,
                layer: layer.to_string(),
            }
        }

        pub fn recreate_path(&self) -> Vec<(u32, usize)> {
            self.path
                .iter()
                .enumerate()
                .map(|(idx, s)| (self.init_step + (idx as u32), *s))
                .collect()
        }
    }

    // agent_id, x, y, step
    pub fn generate_path(
        id: usize,
        path: &mut BinaryHeap<PathSegment>,
        target_mouth: &u16,
    ) -> Vec<Vec<String>> {
        let mut global_path = Vec::new();

        while let Some(segment) = path.pop() {
            segment
                .recreate_path()
                .into_iter()
                .for_each(|(step, position)| {
                    let point = Position::new(position, 627);
                    global_path.push(vec![
                        format!("{id}"),
                        format!("{}", point.x),
                        format!("{}", point.y),
                        format!("{}", segment.layer),
                        format!("{step}"),
                        format!("{target_mouth}"),
                    ]);
                });
        }

        global_path
    }
}
