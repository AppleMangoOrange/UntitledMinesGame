use crate::core::state::BoardInterface;

pub trait Solver {
    fn solve(board: impl BoardInterface);
}
