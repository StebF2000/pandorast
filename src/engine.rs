pub mod matrix {
    use std::{collections::HashMap, fs::File, io::Write};

    use crate::iotwins_model::model::Agent;

    #[derive(Debug)]
    pub struct Grid {
        pub data: Box<[u32]>,
        rows: usize,
        columns: usize,
    }

    impl Grid {
        // Generates an empty matrix. Agent-intended
        pub fn new(size: (usize, usize)) -> Grid {
            Grid {
                data: vec![0_u32; size.0 * size.1].into_boxed_slice(),
                rows: size.0,
                columns: size.1,
            }
        }

        // Load blueprint from resources
        pub fn load_layer(path: &str) -> Grid {
            let raster = raster::open(path).unwrap();

            let image: Vec<u32> = raster.bytes.into_iter().map(|x| x as u32).collect();

            Grid {
                data: image.into_boxed_slice(),
                rows: raster.height as usize,
                columns: raster.width as usize,
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
}
