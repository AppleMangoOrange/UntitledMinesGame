pub mod st;

// use crate::core::solver::Solver;

pub trait Generator {
    fn set_num_perturbs();
    fn get_num_perturbs();
    fn generate();
}
