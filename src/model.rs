pub mod model {

    use std::collections::HashMap;
    use std::error::Error;
    use serde::{Deserialize};

    #[derive(Debug, Deserialize, Clone, Copy)]
    pub struct AgentStats {
        min_vision: u8,
        max_vision: u8,
        min_velocity: u8,
        max_velocity: u8,
        min_age: u8,
        max_age: u8,
        porv_tourist: f32,
        min_wall_distance: u8,
        max_wall_distance: u8,
        min_agent_distance: u8,
        max_agent_distance: u8,
        max_distance_b_agents: u8,
        prov_follow: u8,
        prov_museum: u8,
    }

    #[derive(Debug, Deserialize, Clone, Copy)]
    pub struct Coeffs {
        calpha: f32,
        cbeta: f32,
        cdelta: f32,
        csigma: f32,
        ualpha: f32,
        ubeta: f32,
        udelta: f32,
        usigma: f32,
    }

    #[derive(Debug, Deserialize)]
    pub struct Topology {
        layout_pb: String,
        layout_p05: String,
        layout_p1: String,
        layout_p15: String,
        layout_p2: String,
        layout_p3: String,
        layout_p35: String,
        layout_p4: String,
        layout_p5: String,
        layout_s1: String,
    }

    impl Topology {
        pub fn layers (&self) -> HashMap<&str, &str> {
            HashMap::from([
                ("pb", self.layout_pb.as_str()),
                ("p05", self.layout_p05.as_str()),
                ("p1", self.layout_p1.as_str()),
                ("p15", self.layout_p15.as_str()),
                ("p2", self.layout_p2.as_str()),
                ("p3", self.layout_p3.as_str()),
                ("p35", self.layout_p35.as_str()),
                ("p4", self.layout_p4.as_str()),
                ("p5", self.layout_p5.as_str())
            ])
        }
    }

    #[derive(Debug, Deserialize)]
    pub struct Venue {
        pub gates_info: String,
        pub mouths_info: String,
        pub arrivals_info_csv: String,
    }

    #[derive(Debug, Deserialize, Clone, Copy)]
    pub struct Match {
        match_start: f32,
        seconds_per_step: f32,
        distribute_agents_along_minutes: bool,
    }

    #[derive(Debug, Deserialize)]
    struct Gate {
        gate: String,
        x: u32,
        y: u32,
    }

    pub fn load_gates (path: String) -> Result<HashMap<String, Vec<usize>>, Box<dyn Error>>{

        let mut gates: HashMap<String, Vec<usize>> = HashMap::new();

        let mut reader = csv::Reader::from_path(path)?;

        for result in reader.deserialize() {
            let record: Gate = result?;
            
            match gates.get_mut(&record.gate) {
                Some(gate) => {
                    gate.push((627 * record.x + record.y) as usize);
                },
                None => {
                    gates.insert(record.gate, vec![(627 * record.x + record.y) as usize]);
                }
            }
        }

        Ok(gates)
    }
}

pub mod world {}
