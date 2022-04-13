pub mod matrix {
    use image::io::Reader as ImageReader;
    use serde::{Deserialize, Serialize};
    use std::hash::{Hash, Hasher};
    use std::{collections::HashMap, fs::File, io::Write};

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

            match row_pos {
                0 => {
                    // First row
                    Vec::from([
                        position - 1,
                        position + 1,
                        position + (limit - 1),
                        position + limit,
                        position + (limit + 1),
                    ])
                }
                627 => {
                    // Last row
                    Vec::from([
                        position - (limit + 1),
                        position - limit,
                        position - (limit - 1),
                        position - 1,
                        position + 1,
                    ])
                }
                _ => Vec::from([
                    // Any other row
                    position - (limit + 1),
                    position - limit,
                    position - (limit - 1),
                    position - 1,
                    position + 1,
                    position + (limit - 1),
                    position + limit,
                    position + (limit + 1),
                ]),
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

        // Generates an empty matrix. Agent-intended
        pub fn new(size: (usize, usize)) -> Matrix<u64> {
            Matrix {
                data: vec![0_u64; (size.0 * size.1) as usize],
                n_rows: size.0,
            }
        }

        pub fn write_data(&self) {
            let data: Vec<String> = self.data.iter().map(|n| n.to_string()).collect();

            let mut file = File::create("test").expect("[ERROR]");

            writeln!(file, "{}", data.join(", ")).expect("[Err]");
        }
    }

    #[derive(Clone, Copy, Serialize, Deserialize, Eq)]
    pub struct Position {
        pub x: i32,
        pub y: i32,
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
                x: (idx / 627) as i32,
                y: (idx % 627) as i32,
            }
        }

        pub fn closest(&self, others: Vec<Position>) -> Option<usize> {
            if others.is_empty() {
                return None;
            }

            let mut dist = f32::MAX;
            let mut closest = 0;

            others.into_iter().enumerate().for_each(|(idx, position)| {
                // Fast euclidean distance
                let d = ((i32::pow(self.x - position.x, 2) + i32::pow(self.y - position.y, 2))
                    as f32)
                    .sqrt();

                if d < dist {
                    dist = d;
                    closest = idx;
                }
            });
            // 10 pixels of distance
            match dist < 10.0 {
                true => Some(closest),
                false => None,
            }
        }

        pub fn middle(data: Vec<Position>) -> Position {
            let mut x = 0;
            let mut y = 0;

            data.iter().for_each(|p| {
                x += p.x;
                y += p.y;
            });

            Position {
                x: x / data.len() as i32,
                y: y / data.len() as i32,
            }
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

    fn movements(position: usize, gt: &Matrix<u8>) -> Vec<usize> {
        let row_pos = position / gt.n_rows;

        let mut positions = Vec::with_capacity(4);

        match row_pos {
            0 => {
                positions = Vec::from([]);
            }
            627 => {
                positions = Vec::from([]);
            }
            _ => {
                positions = Vec::from([
                    position - gt.n_rows,
                    position - 1,
                    position + 1,
                    position + gt.n_rows,
                ]);
            }
        }

        // Remove out of matrix and walls position (inplace)
        positions.retain(|pos| -> bool { pos < &(gt.n_rows * gt.n_rows) && gt.data[*pos] != 1 });

        positions
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

pub mod routes {

    use std::{collections::HashMap, fs::File, io::BufWriter};

    use bincode::serialize_into;
    use dashmap::DashMap;
    use serde::{Deserialize, Serialize};

    // Allows for parallel insertion using rayon
    #[derive(Clone)]
    pub struct ConcurrentHashMap {
        pub routes: DashMap<String, DashMap<usize, DashMap<usize, Vec<usize>>>>,
    }

    impl ConcurrentHashMap {
        // Returns new empty route HashMap with concurrent insertion
        pub fn new() -> Self {
            ConcurrentHashMap {
                routes: DashMap::new(),
            }
        }

        // Generate a standard HashMap from Concurrent (only for saving purposes)
        pub fn convert_concurrent(&self) -> DualHashMap {
            let mut map: DualHashMap = DualHashMap::new();

            for (floor, outer) in self.routes.clone() {
                for (p1, inner) in outer {
                    for (p2, path) in inner {
                        map.insert(&floor, p1, p2, path);
                    }
                }
            }
            map
        }

        pub fn insert(&self, layer: String, origin: usize, destination: usize, path: Vec<usize>) {
            match self.routes.get(&layer) {
                Some(layer) => {
                    match layer.get(&origin) {
                        // Origin is present, direct insertion
                        Some(mouth) => {
                            mouth.insert(destination, path);
                        }
                        None => {
                            match layer.get(&destination) {
                                // Destination is present, inverse insertion
                                Some(mouth) => {
                                    mouth.insert(origin, path.into_iter().rev().collect());
                                }
                                None => {
                                    // Layer exists, but no mouth is present
                                    let dest = DashMap::new();
                                    dest.insert(destination, path);
                                    layer.insert(origin, dest);
                                }
                            }
                        }
                    }
                }
                None => {
                    // Layer does not exist
                    let floor = DashMap::new();
                    let mouth = DashMap::new();

                    mouth.insert(destination, path);
                    floor.insert(origin, mouth);

                    self.routes.insert(layer, floor);
                }
            }
        }

        pub fn get(&self, layer: String, origin: usize, destination: usize) -> Option<Vec<usize>> {
            match self.routes.get(&layer) {
                Some(layer) => {
                    // Layer exists
                    match layer.get(&origin) {
                        // Origin exists
                        Some(mouth) => mouth.get(&destination).map(|path| path.to_vec()),
                        None => {
                            match layer.get(&destination) {
                                // Origin does not exist, destination does
                                Some(mouth) => mouth.get(&origin).map(|path| path.to_vec()),
                                None => None, // Layer exists, origin and destination does not
                            }
                        }
                    }
                }
                None => None, // Layer does not exist
            }
        }
    }

    // Convert a standard HashMap to concurrent one
    pub fn convert_standard(map: DualHashMap) -> ConcurrentHashMap {
        let mut concurrent = ConcurrentHashMap::new();

        for (layer, floor) in map.data {
            for (p1, outer) in floor {
                for (p2, path) in outer {
                    concurrent.insert(layer.to_string(), p1, p2, path)
                }
            }
        }

        concurrent
    }

    #[derive(Serialize, Deserialize)]
    pub struct DualHashMap {
        // Layer -> Origin -> Destination -> path
        data: HashMap<String, HashMap<usize, HashMap<usize, Vec<usize>>>>,
    }

    impl DualHashMap {
        pub fn get(&self, layer: String, p1: usize, p2: usize) -> Option<Vec<usize>> {
            match self.data.get(&layer) {
                Some(layer) => {
                    match layer.get(&p1) {
                        // Forward path
                        Some(mouth) => mouth.get(&p2).map(|path| path.to_vec()),
                        None => {
                            match layer.get(&p2) {
                                // Inverse path
                                Some(mouth) => mouth
                                    .get(&p1)
                                    .map(|path| path.clone().into_iter().rev().collect()),
                                None => None,
                            }
                        }
                    }
                }
                None => None,
            }
        }

        pub fn insert(&mut self, floor: &str, p1: usize, p2: usize, path: Vec<usize>) {
            match self.data.get_mut(floor) {
                Some(layer) => {
                    match layer.get_mut(&p1) {
                        // Forward order
                        Some(outer) => {
                            match outer.get(&p2) {
                                Some(_) => {
                                    // Already exists this path (is overwritted)
                                    outer.insert(p2, path);
                                }
                                None => {
                                    outer.insert(p2, path);
                                }
                            }
                        }
                        None => match layer.get_mut(&p2) {
                            // Backwards order
                            Some(inner) => {
                                let rev_path: Vec<usize> = path.into_iter().rev().collect();

                                inner.insert(p1, rev_path);
                            }
                            None => {
                                // It is not present, forward order by default
                                let inner: HashMap<usize, Vec<usize>> = HashMap::from([(p2, path)]);

                                layer.insert(p1, inner);
                            }
                        },
                    }
                }
                None => {
                    let inner: HashMap<usize, Vec<usize>> = HashMap::from([(p2, path)]);
                    let outer: HashMap<usize, HashMap<usize, Vec<usize>>> =
                        HashMap::from([(p1, inner)]);

                    self.data.insert(floor.to_string(), outer);
                }
            }
        }

        pub fn new() -> DualHashMap {
            let data: HashMap<String, HashMap<usize, HashMap<usize, Vec<usize>>>> = HashMap::new();

            DualHashMap { data }
        }

        // pub fn save(&self, path: String) {
        //     let file = BufWriter::new(File::create(path).unwrap());

        //     serialize_into(file, self).expect("[ERROR] Cannot write data");
        // }
    }
}
