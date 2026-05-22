use mines_core::core::generator::st::*;
use mines_core::core::state::grid::Board;
use rand::TryRng;
use rand::rngs::SysRng;

pub fn main() {
    env_logger::try_init().unwrap_or_default();
    let _ = test_generation();
    let _ = test_solve();
}

fn test_generation() -> Result<(), &'static str> {
    let mut rng = SysRng::default();
    const WIDTH: usize = 32;
    const HEIGHT: usize = 16;
    const DENSITY: f64 = 0.4f64;
    let start = (WIDTH / 2, HEIGHT / 2);
    let (grid, solvable) = new_with_mines(
        WIDTH,
        HEIGHT,
        ((WIDTH * HEIGHT) as f64 * DENSITY).round() as usize,
        true,
        start,
        rng.try_next_u64().expect("Cannot generate u64."),
    );

    println!("Got solvable grid: {solvable}");
    println!("Solved grid: {grid}");
    let board = grid.clone_reset(start);
    println!("New board: {board}");
    Ok(())
}

fn test_solve() {
    let board = "100010001110111001100001000110000001110000110111000111000101001100010001011100011";
    let grid = &mut Board::from_mines(9, 9, (4, 4), board.chars().map(|c| c == '1').collect());
    Solver::solve_grid(grid, 35).unwrap();
    println!("Grid: {}", grid);
}
