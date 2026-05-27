use godot::prelude::*;
use log::{debug, info};
use rand::{SeedableRng, rngs::Xoshiro128PlusPlus};

use crate::core::{
    generator::{Generator, st::StGenerator},
    solver::st::StSolver,
    state::{
        BoardError, BoardInterface, Visibility,
        grid::{self, Board},
    },
};

#[derive(GodotClass)]
#[class(no_init, base=Node2D)]
pub struct MinesCore {
    base: Base<Node2D>,
    board: grid::Board,
    #[var(no_set)]
    total_mines: i64,
    #[var]
    revealed: i64,
}

#[godot_api]
impl MinesCore {
    #[signal]
    fn game_lost(x: i64, y: i64);

    #[signal]
    fn game_won(x: i64, y: i64);

    #[signal]
    fn cell_updated(x: i64, y: i64);

    #[func]
    pub fn from_params(
        seed: i64,
        width: i64,
        height: i64,
        num_mines: i64,
        _solvable: bool,
        start_x: i64,
        start_y: i64,
    ) -> Gd<Self> {
        let mut rng = Xoshiro128PlusPlus::seed_from_u64(seed as u64);
        let start = (start_x as usize, start_y as usize);
        let mut board = grid::Board::new_random(
            width as usize,
            height as usize,
            start,
            &mut rng,
            num_mines as usize,
        );
        info!("Board of size {width}x{height} initialised.");
        let mut generator = StGenerator::new(&mut board, 500);
        let _ = generator
            .generate::<StSolver<Board>>(start, &mut rng)
            .expect("Failed board generation.");
        board.reset(start);
        let mut ret = Gd::from_init_fn(|base| Self {
            total_mines: board.get_num_mines() as i64,
            base,
            board,
            revealed: 0,
        });

        {
            let mut game = ret.bind_mut();
            for c in game.board.iter_cells() {
                if game.board.peek(c) != Visibility::Revealed {
                    continue;
                }
                if let Some(hint) = game.board.get_hint(c)
                    && hint == 0
                {
                    game.chord(c.0 as i64, c.1 as i64);
                }
            }
            game.revealed = game.board.count_cells(Visibility::Revealed) as i64
        }

        ret
    }

    #[func]
    pub fn open(&mut self, x: i64, y: i64) -> i8 {
        return if let Ok(hint) = self.board.open((x as usize, y as usize)) {
            debug!("Opened cell at ({x}, {y}).");
            self.revealed += 1;
            self.signals().cell_updated().emit(x, y);
            if self.revealed as usize == self.board.len() - self.total_mines as usize {
                debug!("Game won.");
                self.board.iter_cells().for_each(|c| {
                    if self.board.peek(c) == Visibility::Hidden {
                        self.flag(c.0 as i64, c.1 as i64)
                    }
                });
                self.signals().game_won().emit(x, y)
            } else if hint == 0 {
                self.chord(x, y);
            }
            hint as i8
        } else {
            self.signals().game_lost().emit(x, y);
            -1
        };
    }

    #[func]
    pub fn flag(&mut self, x: i64, y: i64) {
        let coords = (x as usize, y as usize);
        match self.board.peek(coords) {
            Visibility::Hidden => match self.board.flag(coords) {
                Ok(()) | Err(BoardError::BadFlag) => self.signals().cell_updated().emit(x, y),
                Err(_) => (),
            },
            Visibility::Flagged => match self.board.undo_flag(coords) {
                Ok(()) => self.signals().cell_updated().emit(x, y),
                Err(_) => (),
            },
            Visibility::Revealed => (),
        }
    }

    #[func]
    pub fn sprite(&mut self, x: i64, y: i64) -> u8 {
        let (x, y) = (x as usize, y as usize);
        match self.board.peek((x, y)) {
            Visibility::Flagged => 10,
            Visibility::Hidden => 9,
            Visibility::Revealed => self.board.get_hint((x, y)).unwrap() as u8,
        }
    }

    #[func]
    pub fn chord(&mut self, x: i64, y: i64) -> bool {
        let (ix, iy) = (x as isize, y as isize);
        let (ux, uy) = (x as usize, y as usize);
        let Some(hint) = self.board.get_hint((ux, uy)) else {
            return false;
        };
        let neighbours: Vec<(usize, usize)> = self
            .board
            .get_region((ix - 1, iy - 1)..=(ix + 1, iy + 1))
            .flatten()
            .filter(|&c| self.board.peek(c) != Visibility::Revealed)
            .collect();
        let num_flagged = neighbours
            .iter()
            .filter(|&&c| self.board.peek(c) == Visibility::Flagged)
            .count();
        if hint == num_flagged {
            neighbours.iter().for_each(|&c| {
                if self.board.peek(c) == Visibility::Hidden {
                    let _ = self.open(c.0 as i64, c.1 as i64);
                }
            });
            true
        } else if hint == neighbours.len() {
            neighbours.iter().for_each(|&c| {
                if self.board.peek(c) != Visibility::Flagged {
                    let _ = self.flag(c.0 as i64, c.1 as i64);
                }
            });
            true
        } else {
            false
        }
    }
}
