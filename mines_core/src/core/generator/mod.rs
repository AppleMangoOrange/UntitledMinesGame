pub mod st;

use log::{debug, info, warn};

use crate::core::{
    solver::{Constraint, Solver},
    state::BoardInterface,
};

pub trait Generator<'a, B>
where
    B: BoardInterface,
{
    fn new(board: &'a mut B, max_perturbs: usize) -> Self;
    fn get_max_perturbs(&mut self) -> &mut usize;
    fn get_board(&mut self) -> &mut B;
    fn perturb(
        &mut self,
        constraints: Vec<&Constraint<B::Coords>>,
        start: B::Coords,
        rng: &mut impl rand::Rng,
    );
    /// Generates a non-guessing solvable board using the internal solver. Returns Err(()) if
    /// reached maximum perturbations. Returns Ok(num perturbations) if generated successfully.
    fn generate<S>(&mut self, start: B::Coords, rng: &mut impl rand::Rng) -> Result<usize, ()>
    where
        S: Solver<B>,
    {
        info!("Initial grid: {:?}", self.get_board());
        let mut num_perturbs = 0;
        while let Err(constraints) = S::solve(self.get_board()) {
            if num_perturbs >= *self.get_max_perturbs() {
                warn!(
                    "Solver failed and reached maximum perturbations. Returning unsolvable grid."
                );
                return Err(());
            }
            debug!("Running perturbation no. {}", num_perturbs);
            let all_constraints = constraints.iter().flatten().collect();
            self.perturb(all_constraints, start.clone(), rng);
            self.get_board().reset(start.clone());
            num_perturbs += 1;
        }
        info!("Generation completed in {} perturbations.", num_perturbs);

        Ok(num_perturbs)
    }
}

// /// Generates a Game with num_mines mines of width x height dimensions using `seed` in
// /// Xoshiro128PlusPlus. `solvable` decides whether the grid is random or deducible. Returns
// /// (grid, solvable) as solvable generation can fail.
// pub fn new_with_mines<'a, S>(
//     width: usize,
//     height: usize,
//     num_mines: usize,
//     solvable: bool,
//     start: (usize, usize),
//     seed: u64,
// ) -> (Board, bool)
// where
//     S: Solver<'a, Board, Coords>,
// {
//     info!("Generating new game with seed {seed}");
//     let mut random = rngs::Xoshiro128PlusPlus::seed_from_u64(seed);
//     let mut grid = Board::new_random(width, height, start, &mut random, num_mines);

//     if solvable {
//         let res = {
//             let mut solver = S::new(&mut grid, num_mines);
//             solver.solve_generate(start, &mut random)
//         };
//         match res {
//             Ok(()) => (grid, true),
//             Err(()) => (grid, false),
//         }
//     } else {
//         (grid, false)
//     }
// }
