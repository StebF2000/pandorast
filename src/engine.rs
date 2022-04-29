pub mod matrix {
    use image::io::Reader as ImageReader;
    use serde::{Deserialize, Serialize};
    use std::collections::HashMap;
    use std::hash::{Hash, Hasher};

    #[derive(Clone)]
    pub struct Matrix<T> {
        pub data: Vec<T>,
        pub n_rows: usize, // For position purposes
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

            let row_pos = position / limit;
            let col_pos = position % limit;

            match row_pos {
                // First row
                0 => match col_pos {
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
                626 => match col_pos {
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
                _ => match col_pos {
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

            image.save("test.png").expect("");

            Matrix {
                data: image.as_raw().to_vec(),
                n_rows: image.height() as usize,
            }
        }
    }

    #[derive(Clone, Copy, Eq, Serialize, Deserialize)]
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
        pub fn new(idx: usize) -> Position {
            Position {
                x: (idx / 627),
                y: (idx % 627),
            }
        }

        pub fn closest(&self, others: Vec<Position>) -> Option<usize> {
            if others.is_empty() {
                return None;
            }

            let mut dist = i32::MAX;
            let mut closest = 0;

            others.into_iter().enumerate().for_each(|(idx, position)| {
                let d = Position::distance(self, &position);

                if d < dist {
                    dist = d;
                    closest = idx;
                }
            });
            // 10 pixels of distance
            match dist < 10_i32.pow(2) {
                true => Some(closest),
                false => None,
            }
        }

        pub fn middle_location(data: &[usize]) -> Position {
            let p: Vec<Position> = data.iter().map(|idx| Position::new(*idx)).collect();
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
        /// Euclidean distance
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
    pub fn a_star(gt: &Matrix<u8>, origin: usize, destination: usize) -> Vec<usize> {
        // Generates  heuristic field (parallel way) the closer you get, the lower is the penalization (like gradient descend)
        let cost_function: Vec<u64> = gt
            .data
            .iter()
            .enumerate()
            .map(|(i, _)| path_finding::heuristic(i, gt.n_rows))
            .collect();

        let mut dist: Vec<u64> = (0..cost_function.len()).map(|_| u64::MAX).collect(); // Initial cost (inf)
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
                return path.into_iter().rev().collect();
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

        // This is for Rust's compiler hapinness; a route will always be find, is an undirected graph
        vec![]
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
        let row_pos = position / gt.n_rows;
        let col_pos = position % gt.n_rows;

        match row_pos {
            // First row
            0 => {
                match col_pos {
                    // Last column item
                    626 => Vec::from([position - 1, position + gt.n_rows])
                        .into_iter()
                        .filter(|pos| pos < &(gt.n_rows * gt.n_rows) && gt.data[*pos] != 1)
                        .collect(),
                    // First column item
                    0 => Vec::from([position + 1, position + gt.n_rows])
                        .into_iter()
                        .filter(|pos| pos < &(gt.n_rows * gt.n_rows) && gt.data[*pos] != 1)
                        .collect(),
                    _ => Vec::from([position + 1, position - 1, position + gt.n_rows])
                        .into_iter()
                        .filter(|pos| pos < &(gt.n_rows * gt.n_rows) && gt.data[*pos] != 1)
                        .collect(),
                }
            }
            // Last row
            626 => {
                match col_pos {
                    // Last column item
                    626 => Vec::from([position - 1, position - gt.n_rows])
                        .into_iter()
                        .filter(|pos| pos < &(gt.n_rows * gt.n_rows) && gt.data[*pos] != 1)
                        .collect(),
                    // First column item
                    0 => Vec::from([position + 1, position - gt.n_rows])
                        .into_iter()
                        .filter(|pos| pos < &(gt.n_rows * gt.n_rows) && gt.data[*pos] != 1)
                        .collect(),
                    _ => Vec::from([position + 1, position - 1, position - gt.n_rows])
                        .into_iter()
                        .filter(|pos| pos < &(gt.n_rows * gt.n_rows) && gt.data[*pos] != 1)
                        .collect(),
                }
            }
            _ => {
                match col_pos {
                    // Last column item
                    626 => Vec::from([position - 1, position + gt.n_rows, position - gt.n_rows])
                        .into_iter()
                        .filter(|pos| pos < &(gt.n_rows * gt.n_rows) && gt.data[*pos] != 1)
                        .collect(),
                    // First column item
                    0 => Vec::from([position + 1, position + gt.n_rows, position - gt.n_rows])
                        .into_iter()
                        .filter(|pos| pos < &(gt.n_rows * gt.n_rows) && gt.data[*pos] != 1)
                        .collect(),
                    _ => Vec::from([
                        position + 1,
                        position - 1,
                        position + gt.n_rows,
                        position - gt.n_rows,
                    ])
                    .into_iter()
                    .filter(|pos| pos < &(gt.n_rows * gt.n_rows) && gt.data[*pos] != 1)
                    .collect(),
                }
            }
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
