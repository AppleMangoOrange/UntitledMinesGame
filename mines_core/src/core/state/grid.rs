use rand::prelude::IteratorRandom;
use std::{
    fmt,
    ops::{self, Bound, Deref, DerefMut},
};

use super::*;

#[derive(Clone)]
pub struct Grid<T> {
    /// Horizontal axis
    width: usize,
    /// Vertical axis
    height: usize,
    /// Row-major 1-d vector of all cells
    data: Vec<T>,
}

impl<T: Clone + Default> Grid<T> {
    /// Create a new grid initialized to all open cells
    pub fn new(width: usize, height: usize) -> Self {
        Self {
            width,
            height,
            data: vec![T::default(); width * height],
        }
    }
}

impl<T> Grid<T> {
    pub fn from_vec(width: usize, height: usize, grid: Vec<T>) -> Self {
        assert_eq!(
            width * height,
            grid.len(),
            "grid size does not match given dimensions"
        );

        Self {
            width,
            height,
            data: grid,
        }
    }

    /// Gets the list of indices of the squares of cells starting from top-left in row-major order,
    /// or None if out of bounds.
    pub fn get_relative_indices<X, Y>(
        &self,
        (x, y): (usize, usize),
        dx_range: X,
        dy_range: Y,
    ) -> impl Iterator<Item = Option<usize>> + use<T, X, Y>
    where
        X: Iterator<Item = isize> + Clone,
        Y: Iterator<Item = isize>,
    {
        let width = self.width;
        let height = self.height;

        dy_range.flat_map(move |dy| {
            dx_range.clone().map(move |dx| {
                let nx = x.checked_add_signed(dx);
                let ny = y.checked_add_signed(dy);
                match (nx, ny) {
                    (Some(valid_x), Some(valid_y)) if valid_x < width && valid_y < height => {
                        Some(Self::to_index_raw(width, valid_x, valid_y))
                    }
                    _ => None,
                }
            })
        })
    }

    /// Gets the list of indicies of adjacent cells starting from top-left
    /// wrapping each row. Wrapper around `get_relative_indices()`.
    #[inline]
    pub fn get_adjacent_indices(
        &self,
        index: usize,
    ) -> impl Iterator<Item = Option<usize>> + use<T> {
        let adjacent_cells = self.get_relative_indices(self.to_coords(index), -1..=1, -1..=1);
        adjacent_cells.filter(move |i| i != &Some(index))
    }

    /// Converts given co-ordinates to grid index without checking for bounds
    #[inline]
    fn to_index_raw(width: usize, x: usize, y: usize) -> usize {
        y * width + x
    }

    /// Converts given co-ordinates to grid index without checking for bounds
    #[inline]
    pub fn to_index(&self, x: usize, y: usize) -> usize {
        y * self.width + x
    }

    /// Converts given index to co-ordinates without checking for bounds
    #[inline]
    pub fn to_coords(&self, index: usize) -> (usize, usize) {
        (index % self.width, index / self.width)
    }

    /// Iterator through each cell of the grid
    #[inline]
    pub fn iter(&self) -> impl Iterator<Item = &T> {
        self.data.iter()
    }

    #[inline]
    pub fn iter_mut(&mut self) -> impl Iterator<Item = &mut T> {
        self.data.iter_mut()
    }

    pub fn enumerate(&self) -> impl Iterator<Item = ((usize, usize), &T)> {
        self.data
            .iter()
            .enumerate()
            .map(|(i, c)| (self.to_coords(i), c))
    }

    #[inline]
    pub fn range(&self) -> std::ops::Range<usize> {
        0..self.width * self.height
    }

    #[inline]
    pub fn contains(&self, (x, y): (usize, usize)) -> bool {
        x < self.width && y < self.height
    }

    #[inline]
    pub fn check_bounds(&self, (x, y): (isize, isize)) -> Option<(usize, usize)> {
        if x < 0 || y < 0 || x > self.width as isize || y > self.height as isize {
            None
        } else {
            Some((x as usize, y as usize))
        }
    }
}

impl<T: fmt::Display> fmt::Display for Grid<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "[")?;
        for row in self.data.chunks(self.width) {
            write!(f, "\t")?;
            for value in row {
                write!(f, "{:10} ", value)?;
            }
            writeln!(f)?;
        }
        writeln!(f, "]")
    }
}

impl<T> ops::Index<usize> for Grid<T> {
    type Output = T;
    #[inline]
    fn index(&self, index: usize) -> &Self::Output {
        &self.data[index]
    }
}

impl<T> ops::IndexMut<usize> for Grid<T> {
    #[inline]
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        &mut self.data[index]
    }
}

impl<T> ops::Index<(usize, usize)> for Grid<T> {
    type Output = T;
    #[inline]
    fn index(&self, (x, y): (usize, usize)) -> &Self::Output {
        let index = self.to_index(x, y);
        &self.data[index]
    }
}

impl<T> ops::IndexMut<(usize, usize)> for Grid<T> {
    #[inline]
    fn index_mut(&mut self, (x, y): (usize, usize)) -> &mut Self::Output {
        let index = self.to_index(x, y);
        &mut self.data[index]
    }
}

// impl<T, I> ops::Index<(I, I)> for Grid<T> where I: SliceIndex<usize> {}

pub struct Board {
    grid: Grid<Cell>,
    num_mines: usize,
}

impl Deref for Board {
    type Target = grid::Grid<Cell>;

    #[inline]
    fn deref(&self) -> &Self::Target {
        &self.grid
    }
}

impl DerefMut for Board {
    #[inline]
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.grid
    }
}

impl Board {
    /// Generate a grid randomly
    pub fn new_random(
        width: usize,
        height: usize,
        start: (usize, usize),
        rng: &mut impl rand::Rng,
        num_mines: usize,
    ) -> Self {
        assert!(
            start.0 < width && start.1 < height,
            "Starting co-ordinates out of bounds."
        );
        let mut board = Board {
            grid: Grid::new(width, height),
            num_mines,
        };

        let mut valid_indices = Vec::with_capacity(board.width() * board.height());
        for i in board.range() {
            let (x, y) = board.to_coords(i);
            if in_safe_area(start, (x, y)) {
                board[i].visibility = Visibility::Revealed;
            } else {
                valid_indices.push(i);
            }
        }

        assert!(
            num_mines <= valid_indices.len(),
            "Not enough valid cells to place mines: {} < {}",
            valid_indices.len(),
            num_mines
        );
        for index in valid_indices.into_iter().sample(rng, num_mines) {
            board[index].is_mine = true;
        }
        board.generate_hints_from_mines();

        board
    }

    /// Generates a Grid out of a boolean list
    pub fn from_mines(width: usize, height: usize, start: (usize, usize), map: Vec<bool>) -> Self {
        assert_eq!(
            width * height,
            map.len(),
            "Grid size does not match given dimensions."
        );

        let mut board: Board = Board {
            grid: Grid {
                width,
                height,
                data: map
                    .iter()
                    .map(|&c| Cell {
                        is_mine: c,
                        adjacent_mines: 0,
                        visibility: Visibility::Hidden,
                    })
                    .collect(),
            },
            num_mines: map.iter().filter(|&&b| b).count(),
        };

        for i in board.range() {
            let (x, y) = board.to_coords(i);

            if start.0.abs_diff(x) <= 1 && start.1.abs_diff(y) <= 1 {
                board[i].visibility = Visibility::Revealed;
                board[i].is_mine = false;
            }
        }
        board.generate_hints_from_mines();

        board
    }

    fn generate_hints_from_mines(&mut self) {
        let grid = &self.grid;
        let mine_indices: Vec<usize> = grid
            .iter()
            .enumerate()
            .filter(|(_, cell)| cell.is_mine)
            .map(|(i, _)| i)
            .collect();
        for mine_i in mine_indices {
            for nbr_i in self.get_adjacent_indices(mine_i) {
                if let Some(nbr_i) = nbr_i
                // Generate hints for mines in case of perturbation
                // && !self.data[nbr_i].is_mine
                {
                    self[nbr_i].adjacent_mines += 1;
                }
            }
        }
    }

    /// Re-calculates the hint for the cell at the given index
    #[inline]
    pub fn compute_mines(&mut self, index: usize) {
        let mines_count = self
            .get_adjacent_indices(index)
            .flatten()
            .filter(|&i| self[i].is_mine)
            .count();
        self[index].adjacent_mines = mines_count as u8;
    }

    #[inline]
    pub fn count_cells(&self, cell_type: Visibility) -> usize {
        let grid = &self.grid;
        grid.iter().filter(|c| c.visibility == cell_type).count()
    }

    /// Returns a new Grid reset back to the initial state. Only the starting area is visible and
    /// rest of the cells are made hidden.
    pub fn clone_reset(&self, start: (usize, usize)) -> Self {
        let grid = &self.grid;
        let mut grid2 = grid.clone();
        grid2.iter_mut().enumerate().for_each(|(i, c)| {
            c.visibility = if in_safe_area(start, self.to_coords(i)) {
                Visibility::Revealed
            } else {
                Visibility::Hidden
            };
        });
        Board {
            grid: grid2,
            num_mines: self.num_mines,
        }
    }
}

impl fmt::Display for Board {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "[")?;
        for row in self.data.chunks(self.width) {
            write!(f, "\t")?;
            for &cell in row {
                write!(
                    f,
                    "{}",
                    match cell.visibility {
                        Visibility::Hidden => "\x1b[47m",
                        Visibility::Revealed => "\x1b[49m",
                        Visibility::Flagged => "\x1b[41m",
                    }
                )?;
                match cell.is_mine {
                    true => write!(f, "\x1b[31mM"),
                    false => write!(f, "\x1b[30m{}", cell.adjacent_mines),
                }?;
                write!(f, "\x1b[0m")?;
            }
            writeln!(f)?;
        }
        writeln!(f, "]")
    }
}

impl fmt::Debug for Board {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "(123; 1: Visibility, 2: Mine, 3: Adjacent) [")?;
        write!(f, "    ")?;
        for i in 0..self.width {
            write!(f, "{i:3x} ")?;
        }
        writeln!(f)?;
        for (i, row) in self.data.chunks(self.width).enumerate() {
            write!(f, "{i:3x} ")?;
            for &cell in row {
                write!(
                    f,
                    "{}",
                    match cell.visibility {
                        Visibility::Hidden => "\x1b[47mH",
                        Visibility::Revealed => "\x1b[49mR",
                        Visibility::Flagged => "\x1b[41mF",
                    }
                )?;
                match cell.is_mine {
                    true => write!(f, "\x1b[31mM"),
                    false => write!(f, "\x1b[30mO"),
                }?;
                write!(f, "{}\x1b[0m ", cell.adjacent_mines)?;
            }
            writeln!(f)?;
        }
        writeln!(f, "]")
    }
}

impl Coordinate for (usize, usize) {
    type Unbounded = (isize, isize);
}

impl BoardInterface for Board {
    type Coords = (usize, usize);

    #[inline]
    fn height(&self) -> usize {
        self.height
    }

    #[inline]
    fn width(&self) -> usize {
        self.width
    }

    #[inline]
    fn len(&self) -> usize {
        self.data.len()
    }

    #[inline]
    fn get_num_mines(&self) -> usize {
        self.num_mines
    }

    fn open(&mut self, coords: Self::Coords) -> Result<usize, BoardError> {
        if coords.0 > self.width || coords.1 > self.height {
            return Err(BoardError::CoordinatesOutOfBounds);
        }
        let cell = &mut self[coords];
        match cell.visibility {
            Visibility::Revealed => Ok(cell.adjacent_mines as usize),
            Visibility::Flagged => Err(BoardError::OpenFlagged),
            Visibility::Hidden => match cell.is_mine {
                true => Err(BoardError::OpenMine),
                false => {
                    cell.visibility = Visibility::Revealed;
                    Ok(cell.adjacent_mines as usize)
                }
            },
        }
    }

    fn flag(&mut self, coords: Self::Coords) -> Result<(), BoardError> {
        if coords.0 > self.width || coords.1 > self.height {
            return Err(BoardError::CoordinatesOutOfBounds);
        }
        let cell = &mut self[coords];
        match cell.visibility {
            Visibility::Revealed => Err(BoardError::FlagRevealed),
            Visibility::Flagged => Ok(()),
            Visibility::Hidden => match cell.is_mine {
                true => {
                    cell.visibility = Visibility::Flagged;
                    Ok(())
                }
                false => Err(BoardError::FlagOpen),
            },
        }
    }

    #[inline]
    fn peek(&self, coords: Self::Coords) -> Visibility {
        let cell = self[coords];
        cell.visibility
    }

    fn get_hint(&self, coords: Self::Coords) -> Option<usize> {
        let cell = self[coords];
        if cell.visibility == Visibility::Revealed {
            Some(cell.adjacent_mines as usize)
        } else {
            None
        }
    }

    fn get_region<R>(&self, range: R) -> impl Iterator<Item = Option<Self::Coords>> + use<R>
    where
        R: RangeBounds<<Self::Coords as Coordinate>::Unbounded>,
    {
        let (start_x, start_y) = match range.start_bound() {
            Bound::Included(s) => *s,
            Bound::Excluded(s) => (s.0 + 1, s.1 + 1),
            Bound::Unbounded => (0, 0),
        };
        let (end_x, end_y) = match range.end_bound() {
            Bound::Included(s) => (s.0 + 1, s.1 + 1),
            Bound::Excluded(s) => *s,
            Bound::Unbounded => (self.width as isize, self.height as isize),
        };
        let width = self.width as isize;
        let height = self.height as isize;

        (start_y..end_y).flat_map(move |y| {
            (start_x..end_x).map(move |x| {
                if x < 0 || y < 0 || x >= width as isize || y >= height as isize {
                    None
                } else {
                    Some((x as usize, y as usize))
                }
            })
        })
    }
}
