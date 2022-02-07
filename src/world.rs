pub mod matrix {
    use std::ops::{Index, IndexMut};

    #[derive(Debug)]
    pub struct Matrix {
        data: Box<[u8]>,
        rows: u64,
        columns: u64,
        layer: String
    }

    impl Matrix {
        pub fn new (size: (u64, u64)) -> Matrix {
            Matrix { 
                data: vec![0, (size.0 * size.1) as u8].into_boxed_slice(),
                rows: size.0,
                columns: size.1,
                layer: String::from("Agents")
            }
        }

        pub fn load_layer (path: String) -> Matrix {

            let topology: Vec<&str> = path.split("_").collect();

            println!("[INFO] Loading layer {layer}", layer = &topology[1]);

            let image = raster::open(path.as_str()).unwrap();

            Matrix { 
                data: Vec::from(image.bytes).into_boxed_slice(),
                rows: image.height as u64,
                columns: image.width as u64,
                layer: String::from(topology[1])
            }
        }
    }
}