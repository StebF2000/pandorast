use rand::{distributions::Uniform, prelude::Distribution};

pub struct Agent {
    id: usize,
    destination: u16,
    target: u16,
    interest: f32,
    steps: u16,
}

impl Agent {
    pub fn new(id: usize, target: u16, between: Uniform<f32>) -> Agent {
        let mut rng = rand::thread_rng();

        Agent {
            id,
            destination: 0,
            target,
            interest: between.sample(&mut rng),
            steps: 0,
        }
    }

    pub fn action(&mut self) {
        todo!()
    }
}
