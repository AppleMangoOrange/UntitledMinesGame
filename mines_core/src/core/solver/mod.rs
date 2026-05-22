use log::debug;
use std::{cmp::Ordering, collections::VecDeque, fmt, ops::IndexMut};

use crate::core::state::grid::Grid;

use super::state::{BoardInterface, Visibility};

pub mod st;

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

pub trait Solver<'a, Board, C> {
    fn new(board: &'a mut Board, options: SolverOptions) -> Self
    where
        Board: BoardInterface<Coords = C>;
    fn solve(&mut self) -> Result<(), ()>;
    /// Add a newly known/changed cell during generation.
    fn add_new_cell(&mut self, coords: C);
    fn get_constraints(&mut self) -> &mut Grid<Vec<Constraint<C>>>;
    /// Deletes all constraints relating to the given coordinates, regardless of whether the given
    /// coordinates are considered in the rules. To be used during generation.
    fn clear_constraints_at(&mut self, coords: C)
    where
        Grid<Vec<Constraint<C>>>: IndexMut<C, Output = Vec<Constraint<C>>>;
    /// Deletes all constraints that consider the given coordinates in the rule. To be used during generation.
    fn clear_constraints_containing(&mut self, coords: C);
    /// Deletes the specific given constraint.
    fn remove_constraint(&mut self, constraint: &Constraint<C>);
}

/// Replacement of `set` in mines.c. 3x3 square of cells storing mine location and count.
#[derive(Clone, Copy, Eq, PartialEq)]
pub struct Constraint<C> {
    // /// top-left x co-ordinate of the 3x3 square
    // pub x: usize,
    // /// top-left y co-ordinate of the 3x3 square
    // pub y: usize,
    pub coords: C,
    /// 9 boolean values indicating mine positions
    pub mask: u16,
    /// Number of undiscovered mines in remaining set
    pub mines: u8,
}

impl<C> Ord for Constraint<C>
where
    C: Ord,
{
    fn cmp(&self, other: &Self) -> Ordering {
        self.coords
            .cmp(&other.coords)
            .then(self.mask.cmp(&other.mask))
    }
}

impl<C> PartialOrd for Constraint<C>
where
    C: PartialOrd,
{
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(
            self.coords
                .partial_cmp(&other.coords)?
                .then(self.mask.cmp(&other.mask)),
        )
    }
}

type Coords = (usize, usize);
impl Constraint<Coords> {
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
            self.coords.0 += 1;
        }
        while self.mask & TOP_EMPTY == 0 {
            self.mask >>= 3;
            self.coords.1 += 1;
        }
        Some(self)
    }

    pub fn from_grid<B>(board: &B, (x, y): Coords) -> Option<Self>
    where
        B: BoardInterface<Coords = Coords>,
    {
        debug!("New constraint centered at ({:#3x}, {:#3x})...", x, y);
        let mut mask = 0u16;
        let mut mines_remaining = board.get_hint((x, y))? as u8;
        debug!("Set contains {mines_remaining} mines.");

        let mut hook_x = x as isize - 1;
        let mut hook_y = y as isize - 1;
        for (bit_i, nbr_c) in board
            .get_region((hook_x, hook_y)..=(hook_x + 2, hook_y + 2))
            .enumerate()
        {
            // debug!("Neighbour {bit_i} has cell? {nbr_c:?}");
            if let Some(nbr_c) = nbr_c {
                match board.peek(nbr_c) {
                    Visibility::Hidden => mask |= 1 << bit_i,
                    Visibility::Flagged => mines_remaining -= 1,
                    _ => (),
                }
                // debug!(
                //     "Neighbour {bit_i} is {}. Remaining mines: {mines_remaining}",
                //     board.peek(nbr_c)
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
            coords: (hook_x as usize, hook_y as usize),
            mask,
            mines: mines_remaining,
        }
        .normalised()
    }

    #[inline]
    fn masked_cells_raw((x, y): Coords, mask: u16) -> impl Iterator<Item = Coords> {
        (0..9)
            .filter(move |i| mask & (1 << i) != 0)
            .map(move |i| (x + i % 3, y + i / 3))
    }

    /// Iterate over co-ordinates of all hidden cells in the mask.
    #[inline]
    pub fn masked_cells(&self) -> impl Iterator<Item = Coords> {
        Self::masked_cells_raw((self.coords.0, self.coords.1), self.mask)
    }

    /// Whether 1. the cell is in the range of this constraint AND 2. the mask
    /// has the bit of this cell set
    #[inline]
    pub fn contains(&self, (x, y): Coords) -> bool {
        // Assuming the co-ordinates are valid for the grid
        if (self.coords.0 + 1).abs_diff(x) > 1 || (self.coords.1 + 1).abs_diff(y) > 1 {
            return false;
        }
        let bit_index = (y - self.coords.1) * 3 + (x - self.coords.0);
        bit_index < 9 && (self.mask & (1 << bit_index) != 0)
    }

    /// Replacement of setmunge. Returns a new mask representing the intersection (or difference)
    /// of two masks, aligned to `self`'s coordinate system. Does not modify existing values.
    pub fn munge(&self, mut other: Constraint<Coords>, diff: bool) -> u16 {
        if self.coords.0.abs_diff(other.coords.0) >= 3
            || self.coords.1.abs_diff(other.coords.1) >= 3
        {
            other.mask = 0;
        } else {
            while other.coords.0 > self.coords.0 {
                other.mask &= !0b100_100_100;
                other.mask <<= 1;
                other.coords.0 -= 1;
            }
            while other.coords.0 < self.coords.0 {
                other.mask &= !0b001_001_001;
                other.mask >>= 1;
                other.coords.0 += 1;
            }
            while other.coords.1 > self.coords.1 {
                other.mask &= !0b111_000_000;
                other.mask <<= 3;
                other.coords.1 -= 1;
            }
            while other.coords.1 < self.coords.1 {
                // These bits will be shifted out anyways
                // other.mask &= !0b000_000_111;
                other.mask >>= 3;
                other.coords.1 += 1;
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
    pub fn into_constraint_groups(
        constraints: Vec<&Constraint<Coords>>,
    ) -> Vec<Vec<&Constraint<Coords>>> {
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

impl<C> fmt::Debug for Constraint<C>
where
    C: fmt::Debug,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Constraint {{ coords: {:?}, mask: {:09b}, hidden mines: {} }}",
            self.coords,
            self.mask.reverse_bits() >> (16 - 9),
            self.mines,
        )
    }
}
