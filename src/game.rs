use std::{error::Error, fmt::Display};

use rayon::iter::{IndexedParallelIterator as _, IntoParallelIterator as _, ParallelIterator};

use crate::N;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Player {
    X,
    O
}

impl Player {
    pub fn other(self) -> Self {
        match self {
            Player::X => Player::O,
            Player::O => Player::X
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Score {
    Win(Player),
    Tie
}

#[derive(Debug, Clone, Copy, Default)]
pub struct State {
    board: [Option<Player>; (N*N) as usize],
    score: Option<Score>
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct InvalidMove;

impl Display for InvalidMove {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("invalid move")
    }
}

impl Error for InvalidMove {}

impl State {
    pub fn score(self) -> Option<Score> {
        self.score
    }
    
    fn check_win(self) -> Option<Score> {
        for y in 0..N {
            let mut p = None;
            for x in 0..N {
                let idx = (x + y * N) as usize;
                if self.board[idx].is_none() {
                    p = None;
                    break;
                }
                if p.is_none() {
                    p = self.board[idx];
                } else if p != self.board[idx] {
                    p = None;
                    break;
                }
            }
            if let Some(p) = p {
                return Some(Score::Win(p))
            }
        }

        for x in 0..N {
            let mut p = None;
            for y in 0..N {
                let idx = (x + y * N) as usize;
                if self.board[idx].is_none() {
                    p = None;
                    break;
                }
                if p.is_none() {
                    p = self.board[idx];
                } else if p != self.board[idx] {
                    p = None;
                    break;
                }
            }
            if let Some(p) = p {
                return Some(Score::Win(p))
            }
        }

        {
            let mut p = None;
            for xy in 0..N {
                let idx = (xy + xy * N) as usize;
                if self.board[idx].is_none() {
                    p = None;
                    break;
                }
                if p.is_none() {
                    p = self.board[idx];
                } else if p != self.board[idx] {
                    p = None;
                    break;
                }
            }
            if let Some(p) = p {
                return Some(Score::Win(p))
            }
        }

        {
            let mut p = None;
            for yx in 0..N {
                let idx = ((N - yx - 1) + yx * N) as usize;
                if self.board[idx].is_none() {
                    p = None;
                    break;
                }
                if p.is_none() {
                    p = self.board[idx];
                } else if p != self.board[idx] {
                    p = None;
                    break;
                }
            }
            if let Some(p) = p {
                return Some(Score::Win(p))
            }
        }

        if self.board.iter().all(Option::is_some) {
            return Some(Score::Tie)
        }

        None
    }

    pub fn board(self) -> [Option<Player>; (N*N) as usize] {
        self.board
    }

    pub fn turn(self) -> Player {
        let (x, o) = self.board.into_iter().fold((0u8, 0u8), |(x, o), p| {
            match p {
                Some(Player::X) => (x + 1, o),
                Some(Player::O) => (x, o + 1),
                None => (x, o)
            }
        });
        if x == o {
            Player::X
        } else {
            assert_eq!(Some(x), o.checked_add(1));
            Player::O
        }
    }

    pub fn do_move(mut self, x: u8, y: u8) -> Result<Self, InvalidMove> {
        if self.score.is_some() {
            return Err(InvalidMove);
        }

        let x = x as u32;
        let y = y as u32;

        if x >= N || y >= N {
            return Err(InvalidMove);
        }

        let idx = (N * y + x) as usize;
        if self.board[idx].is_some() {
            return Err(InvalidMove);
        }

        self.board[idx] = Some(self.turn());
        self.score = self.check_win();
        Ok(self)
    }

    pub fn par_succs(self) -> impl ParallelIterator<Item = (u8, u8)> {
        assert!(self.score().is_none());

        self.board.into_par_iter()
            .enumerate()
            .filter_map(|(i, v)| {
                let i = i as u32;
                v.is_none().then_some(((i % N) as u8, (i / N) as u8))
            })
    }

    #[expect(dead_code)]
    pub fn succs(self) -> impl Iterator<Item = (u8, u8)> {
        assert!(self.score().is_none());

        self.board.into_iter()
            .enumerate()
            .filter_map(|(i, v)| {
                let i = i as u32;
                v.is_none().then_some(((i % N) as u8, (i / N) as u8))
            })
    }
}