use log::{Level, debug, info, log_enabled, warn};
use rand::seq::{IndexedRandom, SliceRandom};
use rand::{RngExt, SeedableRng, rngs};
use std::cmp::{Ord, Ordering};

use crate::core::solver::SolverOptions;
// use crate::core::solver::Solver as Solver1;
use crate::core::{
    solver::{Constraint, Solver},
    state::{BoardInterface, Visibility, grid::Board, in_safe_area},
};

type Coords = (usize, usize);

pub struct Generator<'a, S: Solver<'a, Board, Coords>> {
    board: &'a mut Board,
    solver: S,
    solver_options: SolverOptions,
}

impl<'a, S> Generator<'a, S>
where
    S: Solver<'a, Board, Coords>,
{
    /// Helper function for `perturb()`. Whether all masked cell in the constraint share the exact
    /// same set of revealed neighbours.
    #[inline]
    fn is_symmetric(&self, constraint: Constraint<Coords>) -> bool {
        let masked: Vec<(usize, usize)> = constraint.masked_cells().collect();
        if masked.len() < 2 {
            return false;
        }

        let mut first_neighbours: Option<Vec<usize>> = None;

        for (nx, ny) in masked {
            let n_index = self.board.to_index(nx, ny);
            let mut revealed_neighbours: Vec<usize> = self
                .board
                .get_adjacent_indices(n_index)
                .flatten()
                .filter(|&i| self.board[i].visibility == Visibility::Revealed)
                .collect();

            revealed_neighbours.sort_unstable();

            if let Some(first) = &first_neighbours {
                if revealed_neighbours != *first {
                    return false;
                }
            } else {
                first_neighbours = Some(revealed_neighbours);
            }
        }

        true
    }

    /// Helper function for `perturb()`. Updates the grid according to swapped mines. This function
    /// does not update existing constraints and must be managed by the caller.
    fn update_grid(&mut self, was_mine: &Vec<usize>, was_open: &Vec<usize>) {
        let changes = was_mine
            .iter()
            .map(|&i| (i, -1i8))
            .chain(was_open.iter().map(|&i| (i, 1i8)));

        for (cell_index, delta) in changes {
            for ni in self.board.get_adjacent_indices(cell_index).flatten() {
                self.board[ni].adjacent_mines =
                    self.board[ni]
                        .adjacent_mines
                        .checked_add_signed(delta)
                        .unwrap_or_else(|| {
                            let (x, y) = self.board.to_coords(cell_index);
                            let (nx, ny) = self.board.to_coords(ni);
                            panic!(
                            "Adding/Subtracting out of bounds: tried {} + {} for neighbour {:?} of
                            cell {:?}.",
                            self.board[ni].adjacent_mines, delta, (nx, ny), (x, y)
                        )
                        });
            }
        }
    }

    /// Helper function for `perturb()`. Shuffles hidden (or hidden and flagged) mines in a patch
    /// given patch_size randomly.
    #[inline]
    fn shuffle_patch(
        &mut self,
        start: (usize, usize),
        patch: (usize, usize),
        rng: &mut impl rand::Rng,
        shuffle_flagged: bool,
        &patch_size: &'static usize,
    ) {
        let mut mines_count: usize = 0;
        let mut was_mine = Vec::new();
        let mut was_open = Vec::new();

        let mut pool = Vec::new();
        for dy in 0..patch_size {
            for dx in 0..patch_size {
                let (x, y) = (patch.0 + dx, patch.1 + dy);
                if x >= self.board.width() || y >= self.board.height() {
                    continue;
                }
                if in_safe_area(start, (x, y)) {
                    continue;
                }
                let index = self.board.to_index(x, y);
                let cell = &mut self.board[index];
                if cell.visibility == Visibility::Hidden
                    || (shuffle_flagged && cell.visibility == Visibility::Flagged)
                {
                    pool.push(index);
                    if cell.is_mine {
                        was_mine.push(index);
                        mines_count += 1;
                        cell.is_mine = false;
                        cell.visibility = Visibility::Hidden;
                    }
                }
            }
        }

        debug!(
            "Shuffling {mines_count} mines at patch ({:#5x}, {:#5x}).",
            patch.0, patch.1
        );

        if pool.is_empty() {
            return;
        }

        pool.sample(rng, mines_count).for_each(|&i| {
            if was_mine.contains(&i) {
                was_mine.retain(|&j| j != i);
            } else {
                was_open.push(i);
            }
            self.board[i].is_mine = true;
        });

        self.update_grid(&was_mine, &was_open);

        for y in patch.1.saturating_sub(1)..=(patch.1 + patch_size + 1).min(self.board.height() - 1)
        {
            for x in
                patch.0.saturating_sub(1)..=(patch.0 + patch_size + 1).min(self.board.width() - 1)
            {
                self.solver.clear_constraints_at((x, y));
            }
        }
        for y in patch.1.saturating_sub(3)..=(patch.1 + patch_size + 1).min(self.board.height() - 1)
        {
            for x in
                patch.0.saturating_sub(3)..=(patch.0 + patch_size + 1).min(self.board.width() - 1)
            {
                if self.board[(x, y)].visibility == Visibility::Revealed {
                    self.solver.add_new_cell((x, y));
                }
            }
        }
    }

    /// Helper function for `perturb()`. Converts constraint to trivial by adding/removing mines
    /// before inserting. Adapted from `mineperturb()` in `mines.c`.
    #[inline]
    fn trvialise_constraint(
        &mut self,
        start: (usize, usize),
        constraint: Constraint<Coords>,
        rng: &mut impl rand::Rng,
        &patch_size: &'static usize,
    ) {
        let grid_density =
            self.board.get_num_mines() as f32 / (self.board.width() * self.board.height()) as f32;
        let local_density = {
            let start = 1.5f32 - patch_size as f32 / 2f32;
            let end = patch_size as f32 / 2f32 + 1f32;
            let num_mines = self
                .board
                .get_relative_indices(
                    (constraint.coords.0, constraint.coords.1),
                    start as isize..=end as isize,
                    start as isize..=end as isize,
                )
                .flatten()
                .filter(|&i| self.board[i].is_mine)
                .count();
            num_mines
        } as f32
            / (patch_size * patch_size) as f32;
        let to_saturate = local_density.total_cmp(&grid_density) == Ordering::Less;
        if to_saturate {
            debug!("Adding mines to constraint as local {local_density} < grid {grid_density}.");
        } else {
            debug!(
                "Removing mines from constraint as local {local_density} >= grid {grid_density}."
            );
        }

        let mut priority_pools: [Vec<usize>; 3] = Default::default();
        for i in self.board.range() {
            let (x, y) = self.board.to_coords(i);

            if self.board[i].is_mine != to_saturate
                || in_safe_area(start, (x, y))
                || constraint.contains((x, y))
            {
                continue;
            }

            let pool_index = match self.board[i].visibility {
                Visibility::Hidden => {
                    let is_frontier = self
                        .board
                        .get_adjacent_indices(i)
                        .flatten()
                        .any(|n| self.board[n].visibility == Visibility::Revealed);
                    if is_frontier { 0 } else { 1 }
                }
                Visibility::Revealed | Visibility::Flagged => 2,
            };

            priority_pools[pool_index].push(i);
        }

        let num_change = if to_saturate {
            constraint.mask.count_ones() as usize - constraint.mines as usize
        } else {
            constraint.mines as usize
        };

        let mut selected = Vec::with_capacity(num_change);
        for mut pool in priority_pools {
            if selected.len() >= num_change {
                break;
            }

            pool.shuffle(rng);
            selected.extend(pool.iter().take(num_change - selected.len()));
        }

        let mut was_mine = Vec::new();
        let mut was_open = Vec::new();
        for (x, y) in constraint.masked_cells() {
            let index = self.board.to_index(x, y);
            if self.board[index].is_mine == to_saturate {
                continue;
            }
            let swapped_index: usize = selected
                .pop()
                .expect("Grid has less cells than masked cells in constraint.");
            if log_enabled!(Level::Debug) {
                let (sx, sy) = self.board.to_coords(swapped_index);
                debug!("Swapping ({x:#5x}, {y:#5x}) with ({sx:#5x}, {sy:#5x}).");
            }
            self.board[index].is_mine = to_saturate;
            self.board[swapped_index].is_mine = !to_saturate;
            self.board[swapped_index].visibility = Visibility::Hidden;

            if to_saturate {
                was_mine.push(swapped_index);
                was_open.push(index);
            } else {
                was_mine.push(index);
                was_open.push(swapped_index);
            }
        }
        debug!("{was_mine:?} were mines. {was_open:?} were open.");
        // constraint.mines = constraint
        //     .mines
        //     .checked_add_signed(num_change as i8 * if to_saturate { 1 } else { -1 })
        //     .unwrap_or_else(|| {
        //         panic!("Added/Removed more mines to {constraint:?} than possible ({num_change}).");
        //     });
        // self.create_constraint(constraint);

        self.update_grid(&was_mine, &was_open);

        for &cell_index in was_mine.iter().chain(was_open.iter()) {
            let cell_coords = self.board.to_coords(cell_index);
            debug!(
                "Updating constraints around ({:#5x}, {:#5x}).",
                cell_coords.0, cell_coords.1
            );
            self.solver.clear_constraints_containing(cell_coords);
            self.board
                .get_region(
                    (cell_coords.0 as isize - 2, cell_coords.1 as isize - 2)
                        ..=(cell_coords.0 as isize + 2, cell_coords.1 as isize + 2),
                )
                .flatten()
                .filter(|&i| self.board[i].visibility == Visibility::Revealed)
                .for_each(|i| self.solver.add_new_cell(i));
        }
    }

    /// Called during generation of the board when the solver fails. Modifies the grid
    /// semi-randomly in the hopes of making it solvable.
    fn perturb(&mut self, start: (usize, usize), rng: &mut impl rand::Rng) -> () {
        const PATCH_SIZE: usize = 5;
        let picked_constraint = {
            let constraints = self.solver.get_constraints();
            let all_constraints: Vec<&Constraint<Coords>> = constraints.iter().flatten().collect();
            if all_constraints.len() < 2 {
                None
            } else {
                let &constraint = all_constraints[rng.random_range(0..all_constraints.len())];
                Some(constraint)
            }
        };
        if let Some(constraint) = picked_constraint {
            debug!("Picked {:?}", constraint);
            if self.is_symmetric(constraint) {
                debug!("Picked constraint is symmetric. Swapping mines globally.");
                self.trvialise_constraint(start, constraint, rng, &PATCH_SIZE);
                self.solver.remove_constraint(&constraint);
            } else {
                let patch = (
                    constraint.coords.0.saturating_sub((PATCH_SIZE - 3) / 2),
                    constraint.coords.1.saturating_sub((PATCH_SIZE - 3) / 2),
                );
                self.shuffle_patch(start, patch, rng, picked_constraint.is_none(), &PATCH_SIZE);
            }
        } else {
            debug!("Not enough constraints for trivialisation. Picking patch from frontier.");
            let frontier: Vec<(usize, usize)> = self
                .board
                .enumerate()
                .filter(|&((x, y), c)| {
                    c.visibility == Visibility::Hidden
                        && self
                            .board
                            .get_relative_indices((x, y), -1..=1, -1..=1)
                            .flatten()
                            .any(|n| self.board[n].visibility == Visibility::Hidden)
                    // .any(|n| self.grid[n].visibility == Visibility::Flagged)
                    // TODO: check visibility criteria (caused generating unsolvable boards)
                })
                .map(|((x, y), _c)| (x, y))
                .collect();
            let (x, y) = frontier[rng.random_range(0..frontier.len())];
            let patch = (
                x.saturating_sub(PATCH_SIZE / 2),
                y.saturating_sub(PATCH_SIZE / 2),
            );
            self.shuffle_patch(start, patch, rng, picked_constraint.is_none(), &PATCH_SIZE);
        }
        // if log_enabled!(Level::Debug) {
        //     let all_constraints: Vec<&Constraint<Coords>> = constraints.iter().flatten().collect();
        //     debug!("Constraints after perturbation: {all_constraints:#?}");
        // }
    }

    // TODO: Update to accept Grid object instead.
    /// Generates a non-guessing solvable board using the internal solver. Returns Err(()) if
    /// reached maximum perturbations.
    fn generate(board: Board, start: (usize, usize), rng: &mut impl rand::Rng) -> Result<(), ()> {
        let mut self = Self {
            board,
            solver: S::new(board, options)
        };
        info!("Initial grid: {:?}", self.board);
        let mut num_perturbs = 0;
        while self
            .solver
            .init(&mut self.board, self.solver_options)
            .is_err()
        {
            if num_perturbs >= self.solver_options.max_perturbations {
                warn!(
                    "Solver failed and reached maximum perturbations. Returning unsolvable grid."
                );
                return Err(());
            }
            debug!("Running perturbation no. {}", num_perturbs);
            self.perturb(start, rng);
            num_perturbs += 1;
        }
        info!("Generation completed in {} perturbations.", num_perturbs);

        Ok(())
    }
}

/// Generates a Game with num_mines mines of width x height dimensions using `seed` in
/// Xoshiro128PlusPlus. `solvable` decides whether the grid is random or deducible. Returns
/// (grid, solvable) as solvable generation can fail.
pub fn new_with_mines<'a, S>(
    width: usize,
    height: usize,
    num_mines: usize,
    solvable: bool,
    start: (usize, usize),
    seed: u64,
) -> (Board, bool)
where
    S: Solver<'a, Board, Coords>,
{
    info!("Generating new game with seed {seed}");
    let mut random = rngs::Xoshiro128PlusPlus::seed_from_u64(seed);
    let mut grid = Board::new_random(width, height, start, &mut random, num_mines);

    if solvable {
        let res = {
            let mut solver = S::new(&mut grid, num_mines);
            solver.solve_generate(start, &mut random)
        };
        match res {
            Ok(()) => (grid, true),
            Err(()) => (grid, false),
        }
    } else {
        (grid, false)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::{SeedableRng, rngs::StdRng};

    #[test]
    fn test_generate() {
        env_logger::try_init().unwrap_or_default();
        let mut rng = StdRng::seed_from_u64(1234);
        let width = 32;
        let height = 16;
        let start = (width / 2, height / 2);
        let num_mines = 200;
        let mut grid = Board::new_random(width, height, start, &mut rng, num_mines);
        let mut generator = Solver::new(&mut grid, num_mines);
        generator
            .solve_generate(start, &mut rng)
            .expect("Failed board generation.");

        println!("Generated grid: {grid:?}");
        let mut grid = grid.clone_reset(start);
        Solver::solve_grid(&mut grid, num_mines).unwrap_or_else(|_| {
            panic!("Solving failed after generation. Partially solved: {grid}");
        });
        println!("Aftering solving: {grid:?}");
        assert_eq!(
            grid.count_cells(Visibility::Hidden),
            0,
            "Number of hidden cells {} > 0.",
            grid.count_cells(Visibility::Hidden)
        );
        assert_eq!(
            grid.count_cells(Visibility::Flagged),
            num_mines,
            "Number of flagged cells {} != {num_mines}",
            grid.count_cells(Visibility::Flagged)
        );
    }
}
