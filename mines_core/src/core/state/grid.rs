use crate::core::state::*;
use rand::prelude::IteratorRandom;
use std::{
    fmt,
    ops::{self, Deref, DerefMut},
};

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

pub struct Board(grid::Grid<Cell>);

impl Deref for Board {
    type Target = grid::Grid<Cell>;

    #[inline]
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for Board {
    #[inline]
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
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
        let mut board = Board(Grid::new(width, height));

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

        let mut board: Board = Board(Grid {
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
        });

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
        let Board(grid) = self;
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
    pub fn compute_mines(&mut self, index: usize) {
        let mines_count = self
            .get_adjacent_indices(index)
            .flatten()
            .filter(|&i| self[i].is_mine)
            .count();
        self[index].adjacent_mines = mines_count as u8;
    }

    pub fn count_cells(&self, cell_type: Visibility) -> usize {
        let Board(grid) = self;
        grid.iter().filter(|c| c.visibility == cell_type).count()
    }

    /// Returns a new Grid reset back to the initial state. Only the starting area is visible and
    /// rest of the cells are made hidden.
    pub fn clone_reset(&self, start: (usize, usize)) -> Self {
        let Board(grid) = self;
        let mut grid2 = grid.clone();
        grid2.iter_mut().enumerate().for_each(|(i, c)| {
            c.visibility = if in_safe_area(start, self.to_coords(i)) {
                Visibility::Revealed
            } else {
                Visibility::Hidden
            };
        });
        Board(grid2)
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
    fn get_total_mines(&self) -> usize {
        let Board(grid) = self;
        grid.iter().filter(|c| c.is_mine).count()
    }

    fn open(&mut self, coords: (usize, usize)) -> Result<usize, BoardError> {
        let cell = self[coords];
        match cell.visibility {
            Visibility::Revealed => Ok(cell.adjacent_mines as usize),
            Visibility::Flagged => Err(BoardError::OpenFlagged),
            Visibility::Hidden => match cell.is_mine {
                true => Err(BoardError::OpenMine),
                false => Ok(cell.adjacent_mines as usize),
            },
        }
    }

    fn flag(&mut self, coords: (usize, usize)) -> Result<(), BoardError> {
        let cell = self[coords];
        match cell.visibility {
            Visibility::Revealed => Err(BoardError::FlagRevealed),
            Visibility::Flagged => Ok(()),
            Visibility::Hidden => match cell.is_mine {
                true => Ok(()),
                false => Err(BoardError::FlagOpen),
            },
        }
    }

    fn peek(&self, coords: (usize, usize)) -> Visibility {
        let cell = self[coords];
        cell.visibility
    }

    fn iter_cells(&self) -> impl Iterator<Item = (Visibility, Self::Coords)> {
        (self as &Board).enumerate().map(|(i, c)| (c.visibility, i))
    }
}
