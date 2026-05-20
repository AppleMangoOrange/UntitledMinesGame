use log::{Level, debug, info, log_enabled, warn};
use rand::seq::{IndexedRandom, SliceRandom};
use rand::{RngExt, SeedableRng, rngs};
use std::cmp::{Ord, Ordering, PartialOrd};
use std::collections::VecDeque;
use std::fmt;

// use crate::core::solver::Solver as Solver1;
use crate::core::{
    state::grid::Grid,
    state::in_safe_area,
    state::{BoardInterface, Cell, Visibility},
};

#[derive(Clone, Copy, Eq, PartialEq)]
/// Replacement of `set` in mines.c. 3x3 square of cells storing mine location and count.
struct Constraint {
    /// top-left x co-ordinate of the 3x3 square
    pub x: usize,
    /// top-left y co-ordinate of the 3x3 square
    pub y: usize,
    /// 9 boolean values indicating mine positions
    pub mask: u16,
    /// Number of undiscovered mines in remaining set
    pub mines: u8,
}

impl Ord for Constraint {
    fn cmp(&self, other: &Self) -> Ordering {
        self.y
            .cmp(&other.y)
            .then(self.x.cmp(&other.x))
            .then(self.mask.cmp(&other.mask))
    }
}

impl PartialOrd for Constraint {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Constraint {
    /// Ported from mines.c ss_add()
    pub fn normalised(mut self) -> Option<Self> {
        const NO_HIDDEN: u16 = 0;
        const LEFT_EMPTY: u16 = 0b001_001_001;
        const TOP_EMPTY: u16 = 0b000_000_111;

        if self.mask == NO_HIDDEN {
            return None;
        }
        while self.mask & LEFT_EMPTY == 0 {
            self.mask >>= 1;
            self.x += 1;
        }
        while self.mask & TOP_EMPTY == 0 {
            self.mask >>= 3;
            self.y += 1;
        }
        Some(self)
    }

    pub fn from_grid(grid: &Grid<Cell>, (x, y): (usize, usize)) -> Option<Self> {
        debug!("New constraint centered at ({:#3x}, {:#3x})...", x, y);
        debug!("Grid: {:?}", grid);
        let mut mask = 0u16;
        let mut mines_remaining = grid[(x, y)].adjacent_mines;
        debug!("Set contains {mines_remaining} mines.");

        let mut hook_x = x as isize - 1;
        let mut hook_y = y as isize - 1;

        for (bit_i, nbr_i) in grid
            .get_relative_indices((x, y), -1..=1, -1..=1)
            .enumerate()
        {
            if let Some(nbr_i) = nbr_i {
                match grid[nbr_i].visibility {
                    Visibility::Hidden => mask |= 1 << bit_i,
                    Visibility::Flagged => mines_remaining -= 1,
                    _ => (),
                }
                // debug!(
                //     "Neighbour {bit_i} is {}. Remaining mines: {mines_remaining}",
                //     grid[nbr_i].visibility
                // );
            }
        }
        if hook_x == -1 {
            mask >>= 1;
            hook_x = 0;
        }
        if hook_y == -1 {
            mask >>= 3;
            hook_y = 0;
        }

        Self {
            x: hook_x as usize,
            y: hook_y as usize,
            mask,
            mines: mines_remaining,
        }
        .normalised()
    }

    #[inline]
    fn masked_cells_raw((x, y): (usize, usize), mask: u16) -> impl Iterator<Item = (usize, usize)> {
        (0..9)
            .filter(move |i| mask & (1 << i) != 0)
            .map(move |i| (x + i % 3, y + i / 3))
    }

    /// Iterate over co-ordinates of all hidden cells in the mask.
    #[inline]
    pub fn masked_cells(&self) -> impl Iterator<Item = (usize, usize)> {
        Self::masked_cells_raw((self.x, self.y), self.mask)
    }

    /// Whether 1. the cell is in the range of this constraint AND 2. the mask
    /// has the bit of this cell set
    #[inline]
    pub fn contains(&self, (x, y): (usize, usize)) -> bool {
        // Assuming the co-ordinates are valid for the grid
        if (self.x + 1).abs_diff(x) > 1 || (self.y + 1).abs_diff(y) > 1 {
            return false;
        }
        let bit_index = (y - self.y) * 3 + (x - self.x);
        bit_index < 9 && (self.mask & (1 << bit_index) != 0)
    }

    /// Replacement of setmunge. Returns a new mask representing the intersection (or difference)
    /// of two masks, aligned to `self`'s coordinate system. Does not modify existing values.
    pub fn munge(&self, mut other: Constraint, diff: bool) -> u16 {
        if self.x.abs_diff(other.x) >= 3 || self.y.abs_diff(other.y) >= 3 {
            other.mask = 0;
        } else {
            while other.x > self.x {
                other.mask &= !0b100_100_100;
                other.mask <<= 1;
                other.x -= 1;
            }
            while other.x < self.x {
                other.mask &= !0b001_001_001;
                other.mask >>= 1;
                other.x += 1;
            }
            while other.y > self.y {
                other.mask &= !0b111_000_000;
                other.mask <<= 3;
                other.y -= 1;
            }
            while other.y < self.y {
                // These bits will be shifted out anyways
                // other.mask &= !0b000_000_111;
                other.mask >>= 3;
                other.y += 1;
            }
        }

        if diff {
            other.mask ^= 0b111_111_111;
        }

        self.mask & other.mask
    }

    /// Separates a flat list of constraints to groups of disjoint constraints. Returns a list
    /// containing lists sorted by mine count, then density.
    #[inline]
    pub fn into_constraint_groups(constraints: Vec<&Constraint>) -> Vec<Vec<&Constraint>> {
        let num_constraints = constraints.len();
        let mut visited = vec![false; num_constraints];
        let mut groups = Vec::new();

        for i in 0..num_constraints {
            if visited[i] {
                continue;
            }

            let mut current_group = Vec::new();
            let mut queue = VecDeque::from([i]); // BFS

            // Visit current node
            visited[i] = true;
            while let Some(j) = queue.pop_front() {
                let curr = constraints[j];
                current_group.push(curr);

                // Find connected nodes
                for k in 0..num_constraints {
                    if !visited[k] && curr.munge(*constraints[k], false) != 0 {
                        visited[k] = true;
                        queue.push_back(k); // Add connected nodes to queue
                    }
                }
            }

            current_group.sort_unstable_by(|a, b| {
                // 1. Number of unknown cells (descending)
                b.mask.count_ones().cmp(&a.mask.count_ones()).then_with(|| {
                    // 2. Mines density (descending)
                    let density_a = a.mines as f32 / a.mask.count_ones() as f32;
                    let density_b = b.mines as f32 / b.mask.count_ones() as f32;
                    density_b.partial_cmp(&density_a).unwrap_or(Ordering::Equal)
                })
            });
            groups.push(current_group);
        }

        groups
    }
}

impl fmt::Debug for Constraint {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Constraint {{ x: {:#5x}, y: {:#5x}, mask: {:09b}, hidden mines: {} }}",
            self.x,
            self.y,
            self.mask.reverse_bits() >> (16 - 9),
            self.mines,
        )
    }
}

pub struct SolverOptions {
    pub max_disjoint_union_recursion_depth: usize,
    pub max_perturbations: usize,
}

impl Default for SolverOptions {
    fn default() -> Self {
        SolverOptions {
            max_disjoint_union_recursion_depth: 15,
            max_perturbations: 500,
        }
    }
}

pub struct Solver<'a> {
    grid: &'a mut Grid<Cell>,
    num_mines: usize,
    /// Replacement of squaretodo in mines.c. Queue of index of cells which have not been
    /// considered. Newly updated cells are added here.
    cell_todo: VecDeque<usize>,
    /// Replacement of setstore in mines.c. Collection of constraints. A constraint is created for
    /// every revealed numbered cell. Any mutations to an object here must be mirrored in
    /// `constraints_todo`.
    constraints: Grid<Vec<Constraint>>,
    /// Replacement of set-todo in mines.c. Queue of constraints to work on in phase 2.
    constraints_todo: VecDeque<Constraint>,
    options: SolverOptions,
}

impl<'a> Solver<'a> {
    // Initialises `todo` with all currently revealed cells
    fn init(grid: &'a mut Grid<Cell>, num_mines: usize) -> Self {
        let mut cell_todo = VecDeque::with_capacity(grid.len());

        for (i, &cell) in grid.iter().enumerate() {
            if cell.visibility == Visibility::Revealed {
                cell_todo.push_back(i);
            }
        }

        Self {
            constraints: Grid::new(grid.width(), grid.height()),
            num_mines,
            grid,
            cell_todo,
            constraints_todo: VecDeque::new(),
            options: SolverOptions::default(),
        }
    }

    /// Inserts a constraint after checking for triviality and existing constraints
    fn insert_constraint(&mut self, constraint: Constraint) {
        debug!("Adding constraint: {constraint:?}.");
        let bucket = &mut self.constraints[(constraint.x, constraint.y)];
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

    fn remove_constraint(&mut self, constraint: &Constraint) {
        self.constraints[(constraint.x, constraint.y)].retain(|c| c != constraint);
    }

    fn known_cell(&mut self, (x, y): (usize, usize), cell_type: Visibility) {
        let index = self.grid.to_index(x, y);
        if self.grid[index].visibility != Visibility::Hidden {
            return;
        }
        self.grid[index].visibility = cell_type;
        self.cell_todo.push_back(index);
        debug!(
            "Updated visibility of ({x:#5x}, {y:#5x}) to {} and added to todo queue.",
            cell_type
        )
    }

    /// Helper function for `solve()`. Dequeues the cell todo list, creating constrinats for open
    /// cells and updating existing constraints for open flagged cells. Returns whether any
    /// progress was made.
    #[inline]
    fn solve_dequeue_cell_todo(&mut self) -> bool {
        let mut done_something = false;
        while let Some(index) = self.cell_todo.pop_front() {
            let (x, y) = self.grid.to_coords(index);
            let cell_visibility = self.grid[index].visibility;
            done_something = true;
            // Creates a constraint for newly discovered open cells and manages
            // trivial resolution.
            if cell_visibility == Visibility::Revealed {
                if let Some(constraint) = Constraint::from_grid(self.grid, (x, y)) {
                    debug!("New constraint from cell ({x:#5x}, {y:#5x}).");
                    self.insert_constraint(constraint);
                }
            }

            // check existing constraints for current cell
            let mut to_remove = Vec::new();
            let mut to_insert = Vec::new();

            for c_i in self
                .grid
                .get_relative_indices((x, y), -2..=0, -2..=0)
                .flatten()
            {
                let bucket = &self.constraints[c_i];
                for &constraint in bucket {
                    let bit_index = (y - constraint.y) * 3 + (x - constraint.x);

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
            if !self.constraints[(c1.x, c1.y)].contains(&c1) {
                debug!("Ignoring and removing stale constraint: {c1:?}.");
                self.constraints[(c1.x, c1.y)].retain(|&c| !(c == c1));
                continue;
            }
            let (x, y) = (c1.x, c1.y);

            let mut overlaps = Vec::new();
            for yy in y.saturating_sub(2)..(y + 3).min(self.grid.height()) {
                for xx in x.saturating_sub(2)..(x + 3).min(self.grid.width()) {
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
                    Constraint::masked_cells_raw((c1.x, c1.y), c1_wing)
                        .for_each(|(cx, cy)| self.known_cell((cx, cy), Visibility::Flagged));
                    // All hidden cells in c2 are open
                    Constraint::masked_cells_raw((c2.x, c2.y), c2_wing)
                        .for_each(|(cx, cy)| self.known_cell((cx, cy), Visibility::Revealed));
                } else if c2.mines.wrapping_sub(c1.mines) == c2_count as u8 {
                    debug!(
                        "Wing elimination: {} - {} = {}",
                        c2.mines, c1.mines, c2_count
                    );
                    Constraint::masked_cells_raw((c2.x, c2.y), c2_wing)
                        .for_each(|(cx, cy)| self.known_cell((cx, cy), Visibility::Flagged));
                    Constraint::masked_cells_raw((c1.x, c1.y), c1_wing)
                        .for_each(|(cx, cy)| self.known_cell((cx, cy), Visibility::Revealed));
                } else if c1_count == 0 {
                    debug!("Subset rule: {:?} ⊆ {:?}", c1, c2);
                    // There are c2.mines - c1.mines in c2_wing
                    self.insert_constraint(Constraint {
                        x: c2.x,
                        y: c2.y,
                        mask: c2_wing,
                        mines: c2.mines - c1.mines,
                    });
                } else if c2_count == 0 {
                    debug!("Subset rule: {:?} ⊆ {:?}", c2, c1);
                    let updated = Constraint {
                        x: c1.x,
                        y: c1.y,
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
        constraint_groups: Vec<Vec<&Constraint>>,
        mines_left: usize,
        hidden_left: usize,
    ) -> Option<Vec<Constraint>> {
        fn recurse(
            search_space: &[&Constraint], // Input set of constraints to find a subset from
            cursor: usize, // Index in `search_space` of Constraint under consideration
            selected: &mut Vec<Constraint>, // Current set of selected constraints
            current_mines: usize, // Number of mines covered by `selected`
            current_cells: usize, // Number of cells covered by `selected`
            total_mines_left: usize, // Total number of mines
            total_hidden_left: usize, // Total number of cells
        ) -> Option<Vec<Constraint>> {
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
            let mut selected: Vec<Constraint> = Vec::new();
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
        let mines_left = self.num_mines - self.grid.count_cells(Visibility::Flagged);
        let hidden_left = self.grid.count_cells(Visibility::Hidden);
        if mines_left == 0 || mines_left == hidden_left {
            let new_visibility = if mines_left == 0 {
                Visibility::Revealed
            } else {
                Visibility::Flagged
            };
            debug!("Global trivial resolution: Remaining hidden cells are {new_visibility}.");
            for i in 0..self.grid.len() {
                if !(self.grid[i].visibility == Visibility::Hidden) {
                    continue;
                }
                self.known_cell(self.grid.to_coords(i), new_visibility);
            }
            return true;
        }

        // Disjoin union search
        let all_constraints: Vec<&Constraint> = self.constraints.iter().flatten().collect();
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

            for i in 0..self.grid.len() {
                if !(self.grid[i].visibility == Visibility::Hidden) {
                    continue;
                }
                let coords = self.grid.to_coords(i);
                debug!("Checking ({:#5x}, {:#5x})", coords.0, coords.1);

                let outside_union = disjoint_union.iter().all(|c| !c.contains(coords));
                if outside_union {
                    self.known_cell(self.grid.to_coords(i), new_visibility);
                }
            }

            return true;
        }
        false
    }

    /// Internal solver function
    fn solve(&mut self) -> Result<(), ()> {
        loop {
            if log_enabled!(log::Level::Debug) {
                debug!("Solver-Generator iteration starting...");
                debug!("Todo indices: {:?}", self.cell_todo);
                debug!("Grid: {:?}", self.grid);
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

            let hidden_left = self.grid.count_cells(Visibility::Hidden);
            if !done_something && hidden_left > 0 {
                debug!("Unable to solve grid.");
                break Err(());
            } else if hidden_left == 0 {
                debug!("Solving complete!");
                break Ok(());
            }
        }
    }

    /// Creates a new solver instance and solves `grid` in place.
    pub fn solve_grid(grid: &mut Grid<Cell>, num_mines: usize) -> Result<(), ()> {
        let mut s = Solver::init(grid, num_mines);
        s.solve()
    }

    /// Helper function for `perturb()`. Whether all masked cell in the constraint share the exact same set of revealed neighbours.
    #[inline]
    fn is_symmetric(&self, constraint: Constraint) -> bool {
        let masked: Vec<(usize, usize)> = constraint.masked_cells().collect();
        if masked.len() < 2 {
            return false;
        }

        let mut first_neighbours: Option<Vec<usize>> = None;

        for (nx, ny) in masked {
            let n_index = self.grid.to_index(nx, ny);
            let mut revealed_neighbours: Vec<usize> = self
                .grid
                .get_adjacent_indices(n_index)
                .flatten()
                .filter(|&i| self.grid[i].visibility == Visibility::Revealed)
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
            for ni in self.grid.get_adjacent_indices(cell_index).flatten() {
                self.grid[ni].adjacent_mines = self.grid[ni]
                        .adjacent_mines
                        .checked_add_signed(delta)
                        .unwrap_or_else(|| {
                            let (x, y) = self.grid.to_coords(cell_index);
                            let (nx, ny) = self.grid.to_coords(ni);
                            panic!(
                            "Adding/Subtracting out of bounds: tried {} + {} for neighbour {:?} of cell {:?}.",
                            self.grid[ni].adjacent_mines, delta, (nx, ny), (x, y)
                        )});
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
                if x >= self.grid.width() || y >= self.grid.height() {
                    continue;
                }
                if in_safe_area(start, (x, y)) {
                    continue;
                }
                let index = self.grid.to_index(x, y);
                let cell = &mut self.grid[index];
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
            self.grid[i].is_mine = true;
        });

        self.update_grid(&was_mine, &was_open);

        for y in patch.1.saturating_sub(2)..=(patch.1 + patch_size).min(self.grid.height() - 1) {
            for x in patch.0.saturating_sub(2)..=(patch.0 + patch_size).min(self.grid.width() - 1) {
                self.constraints[(x, y)].clear();
            }
        }
        for y in patch.1.saturating_sub(3)..=(patch.1 + patch_size + 1).min(self.grid.height() - 1)
        {
            for x in
                patch.0.saturating_sub(3)..=(patch.0 + patch_size + 1).min(self.grid.width() - 1)
            {
                let index = self.grid.to_index(x, y);
                if self.grid[index].visibility == Visibility::Revealed {
                    self.cell_todo.push_back(index);
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
        constraint: Constraint,
        rng: &mut impl rand::Rng,
        &patch_size: &'static usize,
    ) {
        let grid_density = self.num_mines as f32 / (self.grid.width() * self.grid.height()) as f32;
        let local_density = {
            let start = 1.5f32 - patch_size as f32 / 2f32;
            let end = patch_size as f32 / 2f32 + 1f32;
            let num_mines = self
                .grid
                .get_relative_indices(
                    (constraint.x, constraint.y),
                    start as isize..=end as isize,
                    start as isize..=end as isize,
                )
                .flatten()
                .filter(|&i| self.grid[i].is_mine)
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
        for i in self.grid.range() {
            let (x, y) = self.grid.to_coords(i);

            if self.grid[i].is_mine != to_saturate
                || in_safe_area(start, (x, y))
                || constraint.contains((x, y))
            {
                continue;
            }

            let pool_index = match self.grid[i].visibility {
                Visibility::Hidden => {
                    let is_frontier = self
                        .grid
                        .get_adjacent_indices(i)
                        .flatten()
                        .any(|n| self.grid[n].visibility == Visibility::Revealed);
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
            let index = self.grid.to_index(x, y);
            if self.grid[index].is_mine == to_saturate {
                continue;
            }
            let swapped_index: usize = selected
                .pop()
                .expect("Grid has less cells than masked cells in constraint.");
            if log_enabled!(Level::Debug) {
                let (sx, sy) = self.grid.to_coords(swapped_index);
                debug!("Swapping ({x:#5x}, {y:#5x}) with ({sx:#5x}, {sy:#5x}).");
            }
            self.grid[index].is_mine = to_saturate;
            self.grid[swapped_index].is_mine = !to_saturate;
            self.grid[swapped_index].visibility = Visibility::Hidden;

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
            let cell_coords = self.grid.to_coords(cell_index);
            debug!(
                "Updating constraints around ({:#5x}, {:#5x}).",
                cell_coords.0, cell_coords.1
            );
            self.grid // Could try iterating through all existing constraints instead.
                .get_relative_indices(cell_coords, -2..=0, -2..=0)
                .flatten()
                .for_each(|i| self.constraints[i].retain(|c| !c.contains(cell_coords)));
            self.grid
                .get_relative_indices(cell_coords, -2..=2, -2..=2)
                .flatten()
                .filter(|&i| self.grid[i].visibility == Visibility::Revealed)
                .for_each(|i| self.cell_todo.push_back(i));
        }
    }

    /// Called during generation of the board when the solver fails. Modifies the grid
    /// semi-randomly in the hopes of making it solvable.
    fn perturb(&mut self, start: (usize, usize), rng: &mut impl rand::Rng) -> () {
        const PATCH_SIZE: usize = 5;
        let all_constraints: Vec<&Constraint> = self.constraints.iter().flatten().collect();
        let pick_constraint = all_constraints.len() > 1;
        if pick_constraint {
            if log_enabled!(log::Level::Debug) {
                debug!("Picking from existing constraints: {:#?}", all_constraints);
            }
            let &constraint = all_constraints[rng.random_range(0..all_constraints.len())];
            debug!("Picked {:?}", constraint);
            if self.is_symmetric(constraint) {
                debug!("Picked constraint is symmetric. Swapping mines globally.");
                self.trvialise_constraint(start, constraint, rng, &PATCH_SIZE);
                self.remove_constraint(&constraint);
            } else {
                let patch = (
                    constraint.x.saturating_sub((PATCH_SIZE - 3) / 2),
                    constraint.y.saturating_sub((PATCH_SIZE - 3) / 2),
                );
                self.shuffle_patch(start, patch, rng, !pick_constraint, &PATCH_SIZE);
            }
        } else {
            debug!("No existing constraints. Picking patch from frontier.");
            let frontier: Vec<(usize, usize)> = self
                .grid
                .enumerate()
                .filter(|&((x, y), c)| {
                    c.visibility == Visibility::Hidden
                        && self
                            .grid
                            .get_relative_indices((x, y), -1..=1, -1..=1)
                            .flatten()
                            .any(|n| self.grid[n].visibility == Visibility::Hidden)
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
            self.shuffle_patch(start, patch, rng, !pick_constraint, &PATCH_SIZE);
        }
        if log_enabled!(Level::Debug) {
            let all_constraints: Vec<&Constraint> = self.constraints.iter().flatten().collect();
            debug!("Constraints after perturbation: {all_constraints:#?}");
        }
    }

    // TODO: Update to accept Grid object instead.
    /// Generates a non-guessing solvable board using the internal solver. Returns Err(()) if
    /// reached maximum perturbations.
    fn solve_generate(
        &mut self,
        start: (usize, usize),
        rng: &mut impl rand::Rng,
    ) -> Result<(), ()> {
        info!("Initial grid: {:?}", self.grid);
        let mut num_perturbs = 0;
        while self.solve().is_err() {
            if num_perturbs >= self.options.max_perturbations {
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
pub fn new_with_mines(
    width: usize,
    height: usize,
    num_mines: usize,
    solvable: bool,
    start: (usize, usize),
    seed: u64,
) -> (Grid<Cell>, bool) {
    info!("Generating new game with seed {seed}");
    let mut random = rngs::Xoshiro128PlusPlus::seed_from_u64(seed);
    let mut grid = Grid::new_random(width, height, start, &mut random, num_mines);

    if solvable {
        let res = {
            let mut solver = Solver::init(&mut grid, num_mines);
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

    /// Construct a grid from string, with `*` as mines and other characters as open cells
    fn grid_from_ascii(grid: &[&str], start: (usize, usize)) -> Grid<Cell> {
        let height = grid.len();
        let width = grid[0].len();
        let mut map = Vec::with_capacity(width * height);

        for r in grid {
            for ch in r.chars() {
                map.push(ch == '*');
            }
        }

        Grid::from_mines(width, height, start, map)
    }

    #[test]
    /// Tests for solving of a grid generated by Simon Thatham's mines.c. This does not check for
    /// advanced constraint resolution (> 2 Constraints).
    fn test_solver() {
        env_logger::try_init().unwrap_or_default();
        let map = [
            "*...*...*",
            "**.***..*",
            "*....*...",
            "**......*",
            "**....**.",
            "***...***",
            "...*.*..*",
            "*...*...*",
            ".***...**",
        ];
        let num_mines = 35;
        let mut grid = grid_from_ascii(&map, (4, 4));
        Solver::solve_grid(&mut grid, num_mines).expect("Failed to solve grid.");
        println!("Grid: {grid}");
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

    #[test]
    fn test_generate() {
        env_logger::try_init().unwrap_or_default();
        let mut rng = StdRng::seed_from_u64(1234);
        let width = 32;
        let height = 16;
        let start = (width / 2, height / 2);
        let num_mines = 200;
        let mut grid = Grid::new_random(width, height, start, &mut rng, num_mines);
        let mut generator = Solver::init(&mut grid, num_mines);
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
