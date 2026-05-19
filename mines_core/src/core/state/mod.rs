pub mod grid;

use std::{fmt, ops::RangeInclusive};

const GUARANTEED_SAFE: (RangeInclusive<isize>, RangeInclusive<isize>) = (-1..=1, 0..=0); // (-1..=1, -1..=1);

#[inline]
pub fn in_safe_area(start: (usize, usize), (x, y): (usize, usize)) -> bool {
    // debug!(
    //     "Whether ({} <= {} <= {}) AND ({} <= {} <= {}).",
    //     (start.0 as isize + GUARANTEED_SAFE.0.start()) as usize,
    //     x,
    //     (start.0 as isize + GUARANTEED_SAFE.0.end()) as usize,
    //     (start.1 as isize + GUARANTEED_SAFE.1.start()) as usize,
    //     y,
    //     (start.1 as isize + GUARANTEED_SAFE.1.end()) as usize
    // );
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

pub enum BoardError {
    /// Trying to open a flagged cell
    OpenFlagged,
    /// Trying to open a mine (lose condition)
    OpenMine,
    /// Trying to flag a revealed cell
    FlagRevealed,
    /// Trying to flag an open hidden cell (not fatal but wrong)
    FlagOpen,
}

/// Acts as an API for Boards to restrict access only to gameplay-accessible data.
pub trait BoardInterface {
    type Coords;

    /// Total number of cells in the board
    fn len(&self) -> usize;
    /// Horizontal axis
    fn width(&self) -> usize;
    /// Vertical axis
    fn height(&self) -> usize;
    fn get_total_mines(&self) -> usize;

    /// Opens the cell at `coords`, returns Ok(hint) or Err(()) if opened a mine.
    fn open(&mut self, coords: Self::Coords) -> Result<usize, BoardError>;
    /// Flags cell at `coords`, returns Ok(()) or Err(()) depending on whether it was correct.
    fn flag(&mut self, coords: Self::Coords) -> Result<(), BoardError>;
    /// Displays the visibility of the cell at `coords`.
    fn peek(&self, coords: Self::Coords) -> Visibility;

    /// Iterates through cells of the Board.
    fn iter(&self) -> impl Iterator<Item = (Visibility, Self::Coords)>;
}
