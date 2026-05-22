use log::{debug, log_enabled};
use std::collections::VecDeque;

use super::{Constraint, Coords, Solver, SolverOptions};
use crate::core::state::{BoardInterface, Visibility, grid::Grid};

pub struct StSolver<'a, B>
where
    B: BoardInterface<Coords = Coords>,
{
    board: &'a mut B,
    num_mines: usize,
    /// Replacement of squaretodo in mines.c. Queue of index of cells which have not been
    /// considered. Newly updated cells are added here.
    cell_todo: VecDeque<Coords>,
    /// Replacement of setstore in mines.c. Collection of constraints. A constraint is created for
    /// every revealed numbered cell. Any mutations to an object here must be mirrored in
    /// `constraints_todo`.
    constraints: Grid<Vec<Constraint<Coords>>>,
    /// Replacement of set-todo in mines.c. Queue of constraints to work on in phase 2.
    constraints_todo: VecDeque<Constraint<Coords>>,
    options: SolverOptions,
}

impl<'a, B> StSolver<'a, B>
where
    B: BoardInterface<Coords = Coords>,
{
    fn init(board: &'a mut B, options: SolverOptions) -> Self {
        let mut cell_todo = VecDeque::with_capacity(board.len());

        for coords in board.iter_cells() {
            if board.peek(coords) == Visibility::Revealed {
                cell_todo.push_back(coords);
            }
        }

        let width = board.width();
        let height = board.height();
        let num_mines = board.get_num_mines();
        Self {
            board,
            num_mines,
            cell_todo,
            constraints: Grid::new(width, height),
            constraints_todo: VecDeque::new(),
            options,
        }
    }

    fn known_cell(&mut self, coords: Coords, cell_type: Visibility) {
        if self.board.peek(coords) != Visibility::Hidden {
            return;
        }
        match cell_type {
            Visibility::Flagged => {
                self.board.flag(coords).expect("Error while flagging cell.");
            }
            Visibility::Revealed => {
                self.board
                    .open(coords)
                    .expect("Error while revealing cell.");
            }
            Visibility::Hidden => (),
        };
        self.cell_todo.push_back(coords);
        debug!(
            "Updated visibility of ({:#5x}, {:#5x}) to {} and added to todo queue.",
            coords.0, coords.1, cell_type
        )
    }

    /// Inserts a constraint after checking for triviality and existing constraints
    fn insert_constraint(&mut self, constraint: Constraint<Coords>) {
        debug!("Adding constraint: {constraint:?}.");
        let bucket = &mut self.constraints[(constraint.coords.0, constraint.coords.1)];
        if bucket.contains(&constraint) {
            return;
        } else if constraint.mines == 0 {
            debug!("Trivial constraint: all hidden are safe.");
            constraint
                .masked_cells()
                .for_each(|(cx, cy)| self.known_cell((cx, cy), Visibility::Revealed));
        } else if constraint.mines == constraint.mask.count_ones() as u8 {
            debug!("Trivial constraint: all hidden are mines.");
            constraint
                .masked_cells()
                .for_each(|(cx, cy)| self.known_cell((cx, cy), Visibility::Flagged));
        } else {
            debug!("Non-trivial constraint: adding to collection.");
            bucket.push(constraint);
            self.constraints_todo.push_back(constraint);
        }
    }

    fn remove_constraint(&mut self, constraint: &Constraint<Coords>) {
        self.constraints[(constraint.coords.0, constraint.coords.1)].retain(|c| c != constraint);
        self.constraints_todo.retain(|c| c != constraint);
    }

    /// Helper function for `solve()`. Dequeues the cell todo list, creating constrinats for open
    /// cells and updating existing constraints for open flagged cells. Returns whether any
    /// progress was made.
    #[inline]
    fn solve_dequeue_cell_todo(&mut self) -> bool {
        let mut done_something = false;
        while let Some((x, y)) = self.cell_todo.pop_front() {
            let cell_visibility = self.board.peek((x, y));
            done_something = true;
            // Creates a constraint for newly discovered open cells and manages
            // trivial resolution.
            if cell_visibility == Visibility::Revealed {
                if let Some(constraint) = Constraint::from_grid(self.board, (x, y)) {
                    debug!("New constraint from cell ({x:#5x}, {y:#5x}).");
                    self.insert_constraint(constraint);
                }
            }

            // check existing constraints for current cell
            let mut to_remove = Vec::new();
            let mut to_insert = Vec::new();

            for c_i in self
                .board
                .get_region((x as isize - 2, y as isize - 2)..=(x as isize, y as isize))
                .flatten()
            {
                let bucket = &self.constraints[c_i];
                for &constraint in bucket {
                    let bit_index = (y - constraint.coords.1) * 3 + (x - constraint.coords.0);

                    if !constraint.contains((x, y)) {
                        continue;
                    }
                    debug!("{constraint:?} contains ({x:#5x}, {y:#5x}). Updating...");
                    to_remove.push(constraint);
                    let mut updated = constraint.clone();
                    updated.mask &= !(1 << bit_index);

                    if cell_visibility == Visibility::Flagged {
                        updated.mines = updated.mines.saturating_sub(1);
                    }

                    if updated.mask != 0 {
                        debug!("Updated constraint: {updated:?}");
                        if let Some(c) = updated.normalised() {
                            to_insert.push(c);
                        }
                    }
                }
            }

            to_remove.iter().for_each(|c| {
                self.remove_constraint(c);
            });
            to_insert.iter().for_each(|&c| {
                self.insert_constraint(c);
            });
        }
        done_something
    }

    /// Helper function for `solve()`.  Dequeues the constraints todo list, checking each against
    /// other overlapping constraints to perform wing analysis. Returns whether any progress was
    /// made.
    #[inline]
    fn solve_dequeue_constraints_todo(&mut self) -> bool {
        let mut done_something = false;
        while let Some(c1) = self.constraints_todo.pop_front() {
            done_something = true;
            if !self.constraints[(c1.coords.0, c1.coords.1)].contains(&c1) {
                debug!("Ignoring and removing stale constraint: {c1:?}.");
                self.constraints[(c1.coords.0, c1.coords.1)].retain(|&c| !(c == c1));
                continue;
            }
            let (x, y) = (c1.coords.0, c1.coords.1);

            let mut overlaps = Vec::new();
            for yy in y.saturating_sub(2)..(y + 3).min(self.board.height()) {
                for xx in x.saturating_sub(2)..(x + 3).min(self.board.width()) {
                    let bucket = &self.constraints[(xx, yy)];
                    for &c2 in bucket {
                        if c1 != c2 && c1.munge(c2, false) != 0 {
                            overlaps.push(c2);
                        }
                    }
                }
            }

            for c2 in overlaps {
                let c1_wing = c1.munge(c2, true);
                let c2_wing = c2.munge(c1, true);
                let c1_count = c1_wing.count_ones();
                let c2_count = c2_wing.count_ones();

                // Wing elimination
                // ABCD
                // ?12?
                // Subset elimination
                // ABC
                // 12?
                // Both apply (taken up by Wing)
                // ABC
                // 11?
                if c1.mines.wrapping_sub(c2.mines) == c1_count as u8 {
                    debug!(
                        "Wing elimination: {} - {} = {}",
                        c1.mines, c2.mines, c1_count
                    );
                    // All hidden cells in c1 are mines
                    Constraint::masked_cells_raw((c1.coords.0, c1.coords.1), c1_wing)
                        .for_each(|(cx, cy)| self.known_cell((cx, cy), Visibility::Flagged));
                    // All hidden cells in c2 are open
                    Constraint::masked_cells_raw((c2.coords.0, c2.coords.1), c2_wing)
                        .for_each(|(cx, cy)| self.known_cell((cx, cy), Visibility::Revealed));
                } else if c2.mines.wrapping_sub(c1.mines) == c2_count as u8 {
                    debug!(
                        "Wing elimination: {} - {} = {}",
                        c2.mines, c1.mines, c2_count
                    );
                    Constraint::masked_cells_raw((c2.coords.0, c2.coords.1), c2_wing)
                        .for_each(|(cx, cy)| self.known_cell((cx, cy), Visibility::Flagged));
                    Constraint::masked_cells_raw((c1.coords.0, c1.coords.1), c1_wing)
                        .for_each(|(cx, cy)| self.known_cell((cx, cy), Visibility::Revealed));
                } else if c1_count == 0 {
                    debug!("Subset rule: {:?} ⊆ {:?}", c1, c2);
                    // There are c2.mines - c1.mines in c2_wing
                    self.insert_constraint(Constraint {
                        coords: (c2.coords.0, c2.coords.1),
                        mask: c2_wing,
                        mines: c2.mines - c1.mines,
                    });
                } else if c2_count == 0 {
                    debug!("Subset rule: {:?} ⊆ {:?}", c2, c1);
                    let updated = Constraint {
                        coords: (c1.coords.0, c1.coords.1),
                        mask: c1_wing,
                        mines: c1.mines - c2.mines,
                    };
                    if let Some(c) = updated.normalised() {
                        self.insert_constraint(c);
                    }
                }
            }
        }

        done_something
    }

    /// Helper function for `solve_total_mines_count()`. Searches for a subset
    /// of constraints that resolve trivially using recursive exhaustive
    /// search. Uses an internal recursive function to do the search.
    #[inline]
    fn find_disjoint_union(
        &self,
        constraint_groups: Vec<Vec<&Constraint<Coords>>>,
        mines_left: usize,
        hidden_left: usize,
    ) -> Option<Vec<Constraint<Coords>>> {
        fn recurse(
            search_space: &[&Constraint<Coords>], // Input set of constraints to find a subset from
            cursor: usize, // Index in `search_space` of Constraint under consideration
            selected: &mut Vec<Constraint<Coords>>, // Current set of selected constraints
            current_mines: usize, // Number of mines covered by `selected`
            current_cells: usize, // Number of cells covered by `selected`
            total_mines_left: usize, // Total number of mines
            total_hidden_left: usize, // Total number of cells
        ) -> Option<Vec<Constraint<Coords>>> {
            // If the number of mines in cells not covered by `selected` is 0 or equal to the
            // remaining number of cells, the cells not in `selected` can be solved trivially.
            let remaining_mines = total_mines_left.checked_sub(current_mines)?;
            let remaining_hidden = total_hidden_left.checked_sub(current_cells)?;
            if remaining_hidden > 0 && (remaining_mines == 0 || remaining_mines == remaining_hidden)
            {
                return Some(selected.clone());
            }

            for i in cursor..search_space.len() {
                let &candidate = search_space[i];
                let is_disjoint = selected.iter().all(|c| c.munge(candidate, false) == 0);
                if !is_disjoint {
                    continue;
                }

                selected.push(candidate);
                if let Some(union) = recurse(
                    search_space,
                    i + 1,
                    selected,
                    current_mines + candidate.mines as usize,
                    current_cells + candidate.mask.count_ones() as usize,
                    total_mines_left,
                    total_hidden_left,
                ) {
                    return Some(union);
                }

                selected.pop();
            }

            None
        }

        for group in constraint_groups {
            let search_space = if group.len() > self.options.max_disjoint_union_recursion_depth {
                &group[..self.options.max_disjoint_union_recursion_depth]
            } else {
                &group[..]
            };
            let mut selected: Vec<Constraint<Coords>> = Vec::new();
            if let Some(union) = recurse(
                search_space,
                0,
                &mut selected,
                0,
                0,
                mines_left,
                hidden_left,
            ) {
                return Some(union);
            }
        }

        None
    }

    /// Helper function for `solve()`.  Considers the total mines count to try and solve the
    /// remaining board. Failing that, attempts to create disjoint sets of constraints to try and
    /// solve the trivial set. Returns whether any progress was made.
    #[inline]
    fn solve_total_mines_count(&mut self) -> bool {
        let mines_left = self.num_mines - self.board.count_cells(Visibility::Flagged);
        let hidden_left = self.board.count_cells(Visibility::Hidden);
        if mines_left == 0 || mines_left == hidden_left {
            let new_visibility = if mines_left == 0 {
                Visibility::Revealed
            } else {
                Visibility::Flagged
            };
            debug!("Global trivial resolution: Remaining hidden cells are {new_visibility}.");
            for c in self.board.iter_cells() {
                if self.board.peek(c) == Visibility::Hidden {
                    self.known_cell(c, new_visibility)
                }
            }
            return true;
        }

        // Disjoint union search
        let all_constraints: Vec<&Constraint<Coords>> = self.constraints.iter().flatten().collect();
        let constraint_groups = Constraint::into_constraint_groups(all_constraints);
        if let Some(disjoint_union) =
            self.find_disjoint_union(constraint_groups, mines_left, hidden_left)
        {
            let union_selected_mines: usize = disjoint_union.iter().map(|c| c.mines as usize).sum();
            let new_visibility = if mines_left - union_selected_mines > 0 {
                Visibility::Flagged
            } else {
                Visibility::Revealed
            };
            debug!(
                "Global resolution by disjoint sets: Disjoint hidden cells outside {disjoint_union:?} are {new_visibility}."
            );

            for coords in self.board.iter_cells() {
                if self.board.peek(coords) != Visibility::Hidden {
                    continue;
                }
                debug!("Checking ({:#5x}, {:#5x})", coords.0, coords.1);

                let outside_union = disjoint_union.iter().all(|c| !c.contains(coords));
                if outside_union {
                    self.known_cell(coords, new_visibility);
                }
            }

            return true;
        }
        false
    }

    fn solver_loop(&mut self) -> Result<(), ()>
    where
        B: BoardInterface<Coords = Coords>,
    {
        loop {
            if log_enabled!(log::Level::Debug) {
                debug!("Solver iteration starting...");
                debug!("Todo indices: {:?}", self.cell_todo);
                debug!("Grid: {:?}", self.board);
            }
            let done_something;
            if !self.cell_todo.is_empty() {
                // Constraint formation + trivial resolution
                debug!("Dequeuing cell_todo to form constraints.");
                done_something = self.solve_dequeue_cell_todo();
            } else if !self.constraints_todo.is_empty() {
                // Wing elimination + subset rule
                debug!("Dequeuing constraints_todo to determine cells.");
                done_something = self.solve_dequeue_constraints_todo()
            } else {
                // Constratint formation <-> elimination loop
                // Only runs when both todo lists are empty
                // Look at total mines; trivial resolution
                debug!("Both todo lists empty. Attempting global resolution.");
                done_something = self.solve_total_mines_count()
            }

            let hidden_left = self.board.count_cells(Visibility::Hidden);
            if !done_something && hidden_left > 0 {
                debug!("Unable to solve grid.");
                break Err(());
            } else if hidden_left == 0 {
                debug!("Solving complete!");
                break Ok(());
            }
        }
    }
}

impl<'a, B> Solver<B> for StSolver<'a, B>
where
    B: BoardInterface<Coords = Coords>,
{
    fn solve_with_options(
        board: &mut B,
        options: SolverOptions,
    ) -> Result<(), Grid<Vec<Constraint<Coords>>>> {
        let mut solver = StSolver::init(board, options);
        match solver.solver_loop() {
            Ok(()) => Ok(()),
            Err(()) => Err(solver.constraints),
        }
    }
}
