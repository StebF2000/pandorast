pub mod matrix {
    use image::io::Reader as ImageReader;
    use serde::{Deserialize, Serialize};
    use std::{collections::HashMap, fs::File, io::Write};

    use crate::iotwins::model::Agent;

    #[derive(Debug, Clone)]
    pub struct Matrix<T> {
        pub data: Vec<T>,
        n_rows: u32,
        n_cols: u32,
    }

    impl Matrix<u64> {
        // Given a HashMap, converts blueprint to a standard form for path finding algorithm to work in
        // Codification should be stablised for obstacles (value = 1)
        pub fn ground_thruth(layer: &Matrix<u8>, codification: HashMap<u8, u8>) -> Matrix<u8> {
            let gt: Vec<u8> = layer
                .data
                .iter()
                .map(|value| {
                    if let Some(important) = codification.get(value) {
                        *important
                    } else {
                        0_u8
                    }
                })
                .collect();

            Matrix {
                data: gt,
                n_rows: layer.n_rows,
                n_cols: layer.n_cols,
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
                n_rows: image.height(),
                n_cols: image.width(),
            }
        }

        // Generates an empty matrix. Agent-intended
        pub fn new(size: (u32, u32)) -> Matrix<u64> {
            Matrix {
                data: vec![0_u64; (size.0 * size.1) as usize],
                n_rows: size.0,
                n_cols: size.1,
            }
        }

        // Update agent position
        pub fn matrix_movement(&mut self, agent: &Agent, position: usize) {
            self.data[agent.position] = 0;
            self.data[position] = agent.id;
        }

        pub fn write_data(&self) {
            let data: Vec<String> = self.data.iter().map(|n| n.to_string()).collect();

            let mut file = File::create("test").expect("[ERROR]");

            writeln!(file, "{}", data.join(", ")).expect("[Err]");
        }
    }

    #[derive(Debug, Clone, Copy, Serialize, Deserialize)]
    pub struct Position {
        pub x: i32,
        pub y: i32,
    }

    impl PartialEq for Position {
        fn eq(&self, other: &Self) -> bool {
            self.x == other.x && self.y == other.y
        }
    }

    impl Position {
        pub fn closest(&self, others: Vec<Position>) -> usize {
            let mut dist = i32::MAX;
            let mut closest = 0;

            others.iter().enumerate().for_each(|(idx, position)| {
                // Fast euclidean distance
                let d = i32::pow(self.x - position.x, 2) + i32::pow(self.y - position.y, 2);

                if d < dist {
                    dist = d;
                    closest = idx;
                }
            });

            closest
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
    use rayon::iter::{IndexedParallelIterator, IntoParallelRefIterator, ParallelIterator};
    use std::cmp::Ordering;
    use std::collections::BinaryHeap;

    // A* algorithm form origin to destination, grid must be squared
    // origin and destination will be supposed to be in grid. Blueprint should be passed.
    pub fn a_star(gt: &[u8], origin: usize, destination: usize, grid_size: usize) -> Vec<usize> {
        // Generates  heuristic field (parallel way) the closer you get, the lower is the penalization (like gradient descend)
        let cost_function: Vec<u64> = gt
            .par_iter()
            .enumerate()
            .map(|(i, _)| heuristic(i, grid_size))
            .collect();

        let mut dist: Vec<u64> = (0..cost_function.len()).map(|_| u64::MAX).collect(); // Initial cost (inf)
        let mut heap = BinaryHeap::new();

        // Cost at origin is None (0)
        dist[origin] = 0;

        heap.push(State {
            cost: 0_u64,               // Initial cost (None)
            position: origin,          // Initial position
            path: Vec::from([origin]), // Starting point
        });

        while let Some(State {
            cost,
            position,
            path,
        }) = heap.pop()
        {
            // Stop at destination
            if position == destination {
                return path;
            }

            // Better route already found
            if cost > dist[position] {
                continue;
            }

            // Coputes new heuristic (inlined for compiler optimization purposes)

            movements(position, grid_size, gt)
                .into_iter()
                .for_each(|pos| {
                    // Cost towards next step (actual cost + step + heuristic)
                    let new_cost = cost + 1_u64 + cost_function[pos];

                    // Next aviable position (node) with current route
                    // If so, add it to the frontier and continue
                    if new_cost < dist[pos] {
                        // Update new cost
                        dist[pos] = new_cost;

                        heap.push(State {
                            cost: new_cost,
                            position: pos,
                            path: Vec::from([pos]),
                        });
                    }
                });
        }

        // This is for Rust's compiler hapinness; a route will always be find, is an undirected graph
        vec![]
    }

    // Basic heuristic function, derivate-like where as you get closer to the target, the cost reduces
    // Diagonal values taking into account, should be adapted for obstacles
    fn heuristic(pos: usize, grid_size: usize) -> u64 {
        let row = pos / grid_size;
        let col = pos % grid_size;

        std::cmp::max(row, col) as u64
    }

    fn movements(position: usize, grid_size: usize, gt: &[u8]) -> Vec<usize> {
        let col_pos = position % grid_size;
        let row_pos = position / grid_size;

        // Get new positons
        let mut positions = vec![
            (grid_size * row_pos + col_pos) - 1,
            (grid_size * row_pos + col_pos) + 1,
            (grid_size + 1) * row_pos + col_pos,
            (grid_size - 1) * row_pos + col_pos,
        ];
        // Remove out of matrix and walls positions
        positions.retain(|pos| (pos < &(grid_size * grid_size)) && (gt[*pos] == 0));

        positions
    }

    // Converting BinaryHeap from max-heap to min-heap (A*)
    // Includes  State, Ord and PartialOrd
    #[derive(Eq, PartialEq)]
    struct State {
        cost: u64,
        position: usize,
        path: Vec<usize>,
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

    use std::{
        collections::HashMap,
        fs::{self, File},
        io::{BufRead, BufReader, BufWriter},
    };

    use bincode::{deserialize_from, serialize_into};
    use dashmap::DashMap;
    use serde::{Deserialize, Serialize};

    // Allows for parallel insertion using rayon
    #[derive(Debug, Clone)]
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
    pub fn convert_standard(map: DualHashMap<usize, usize, Vec<usize>>) -> ConcurrentHashMap {
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

    #[derive(Debug)]
    pub struct DualHashMap<T, U, P> {
        // Layer -> Origin -> Destination -> path
        data: HashMap<String, HashMap<T, HashMap<U, P>>>,
    }

    impl DualHashMap<usize, usize, Vec<usize>> {
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

        pub fn insert(&mut self, floor: &String, p1: usize, p2: usize, path: Vec<usize>) {
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

        pub fn new() -> DualHashMap<usize, usize, Vec<usize>> {
            let data: HashMap<String, HashMap<usize, HashMap<usize, Vec<usize>>>> = HashMap::new();

            DualHashMap { data }
        }
    }

    // Generate a standard HashMap from Concurrent (only for saving purposes)
    pub fn convert_concurrent(
        concurrent: ConcurrentHashMap,
    ) -> DualHashMap<usize, usize, Vec<usize>> {
        let mut map: DualHashMap<usize, usize, Vec<usize>> = DualHashMap::new();

        for (floor, outer) in concurrent.routes {
            for (p1, inner) in outer {
                for (p2, path) in inner {
                    map.insert(&floor, p1, p2, path);
                }
            }
        }
        map
    }

    #[derive(Debug, Deserialize, Serialize)]
    pub struct Save {
        layer: String,
        p1: usize,
        p2: usize,
        path: Vec<usize>,
    }

    pub fn save_paths(data: DualHashMap<usize, usize, Vec<usize>>) {
        let mut paths: Vec<Save> = Vec::new();

        for (layer, floor) in data.data {
            for (p1, outer) in floor {
                for (p2, path) in outer {
                    paths.push(Save {
                        layer: layer.clone(),
                        p1,
                        p2,
                        path,
                    });
                }
            }
        }

        let mut file = BufWriter::new(File::create("resources/routes/routes.bin").unwrap());

        for path in paths {
            serialize_into(&mut file, &path).unwrap();
        }
    }
}
