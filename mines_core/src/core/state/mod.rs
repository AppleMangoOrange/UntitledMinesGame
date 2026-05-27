pub mod grid;

use log::trace;
use std::{
    fmt::{self, Debug},
    ops::{RangeBounds, RangeInclusive},
};

const GUARANTEED_SAFE: (RangeInclusive<isize>, RangeInclusive<isize>) = (-1..=1, 0..=0);
// const GUARANTEED_SAFE: (RangeInclusive<isize>, RangeInclusive<isize>) = (-1..=1, -1..=1);

#[inline]
pub fn in_safe_area(start: (usize, usize), (x, y): (usize, usize)) -> bool {
    trace!(
        "Whether ({} <= {} <= {}) AND ({} <= {} <= {}).",
        (start.0 as isize + GUARANTEED_SAFE.0.start()) as usize,
        x,
        (start.0 as isize + GUARANTEED_SAFE.0.end()) as usize,
        (start.1 as isize + GUARANTEED_SAFE.1.start()) as usize,
        y,
        (start.1 as isize + GUARANTEED_SAFE.1.end()) as usize
    );
    ((start.0 as isize + GUARANTEED_SAFE.0.start()) as usize <= x
        && x <= (start.0 as isize + GUARANTEED_SAFE.0.end()) as usize)
        && ((start.1 as isize + GUARANTEED_SAFE.1.start()) as usize <= y
            && y <= (start.1 as isize + GUARANTEED_SAFE.1.end()) as usize)
}

#[derive(Clone, Copy, PartialEq, Default, Debug)]
pub enum Visibility {
    #[default]
    Hidden,
    Flagged,
    // /// Uncovered by player but marked as `?`. Only applies to non-mines.
    // Question,
    Revealed,
}

impl fmt::Display for Visibility {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}",
            match self {
                Visibility::Hidden => "Hidden",
                Visibility::Flagged => "Flagged",
                // Visibility::Question => "Question",
                Visibility::Revealed => "Revealed",
            }
        )
    }
}

#[derive(Clone, Copy, Default, Debug)]
pub struct Cell {
    pub is_mine: bool,
    pub adjacent_mines: u8,
    pub visibility: Visibility,
}

impl Cell {
    pub fn unconver(&mut self) {
        self.visibility = Visibility::Revealed;
    }

    pub fn flag(&mut self) {
        self.visibility = Visibility::Flagged;
    }
}

#[derive(Debug)]
pub enum BoardError {
    CoordinatesOutOfBounds,
    /// Trying to open a flagged cell (usually ignored)
    OpenFlagged,
    /// Trying to open a mine (lose condition)
    OpenMine,
    /// Trying to flag a revealed cell or unflag a revealed or hidden cell
    UnFlaggable,
    /// Trying to flag an open hidden cell (not fatal but wrong)
    BadFlag,
}

pub trait Coordinate: Clone {
    /// The signed version of this coordinate system
    type Unbounded: Clone;
}

/// Acts as an API for Boards to restrict access only to gameplay-accessible data.
pub trait BoardInterface: Debug {
    type Coords: Coordinate;

    /// Total number of cells in the board
    fn len(&self) -> usize;
    /// Horizontal axis
    fn width(&self) -> usize;
    /// Vertical axis
    fn height(&self) -> usize;
    fn get_num_mines(&self) -> usize;

    /// Opens the cell at `coords`, returns Ok(hint) or Err(()) if invalid.
    fn open(&mut self, coords: Self::Coords) -> Result<usize, BoardError>;
    /// Flags cell at `coords`.
    fn flag(&mut self, coords: Self::Coords) -> Result<(), BoardError>;
    /// Unflags cell at `coords`.
    fn undo_flag(&mut self, coords: Self::Coords) -> Result<(), BoardError>;
    /// Displays the visibility of the cell at `coords`.
    fn peek(&self, coords: Self::Coords) -> Visibility;
    fn get_hint(&self, coords: Self::Coords) -> Option<usize>;
    /// Resets the Grid back to the initial state in-place. Only the starting area is visible and
    /// rest of the cells are made hidden.
    fn reset(&mut self, start: Self::Coords);

    /// Iterates through the given region of the Board.
    fn get_region<R>(&self, range: R) -> impl Iterator<Item = Option<Self::Coords>> + use<Self, R>
    where
        R: RangeBounds<<Self::Coords as Coordinate>::Unbounded>;

    /// Iterates through all cells of the Board.
    #[inline]
    fn iter_cells(&self) -> impl Iterator<Item = Self::Coords> + use<Self> {
        return self.get_region(..).flatten();
    }

    /// Counts cells of a specific type by iterating over the board.
    #[inline]
    fn count_cells(&self, visibility: Visibility) -> usize {
        self.iter_cells()
            .filter(|c| self.peek(c.clone()) == visibility)
            .count()
    }
}
