use abm::grid_2d::Grid2D;

use crate::agent_adapter::AgentAdapter;

/// Represents the main grid containing ants and their location.
/// As for now it serves more of a logging purpose than anything,
/// in future it can be used to run operations on all the ants of the simulation,
/// for example to disable their sprites to be able to focus on the pheromones.
pub struct AntsGrid {
    pub grid: Grid2D<AgentAdapter>,
}

impl AntsGrid {
    pub fn new(width: i64, height: i64) -> AntsGrid {
        AntsGrid {
            grid: Grid2D::new(width, height),
        }
    }
}
