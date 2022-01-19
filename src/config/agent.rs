use serde::{Deserialize};

#[derive(Debug, Deserialize)]
pub(crate) struct Agent {
    min_vision: u8,
    max_vision: u8,
    min_speed: u8,
    max_speed: u8,
    min_age: u8,
    max_age: u8,
    min_wall_dist: u8,
    max_wall_dist: u8,
    min_agent_dist: u8,
    max_agent_dist: u8,
    max_dist_between_agents: u8,
    a_tourist: f32,
    a_follow: u8,
    a_museum: u8,
}
