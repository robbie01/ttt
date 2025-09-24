use std::hash::{BuildHasherDefault, DefaultHasher};

use rayon::iter::ParallelIterator as _;
use scc::HashMap;

use crate::{game::{Player, Score, State}, N};

static MEMO: HashMap<u64, (i8, Option<(u8, u8)>), BuildHasherDefault<DefaultHasher>> = HashMap::with_hasher(BuildHasherDefault::new());

fn densely_pack(board: [Option<Player>; (N*N) as usize], p: Player) -> u64 {
    const {
        assert!(N <= 7);
    }
    let mut m = 0;
    m |= match p {
        Player::X => 0,
        Player::O => 1
    } << 63;
    for (i, v) in board.into_iter().enumerate() {
        if let Some(p) = v {
            m |= match p {
                Player::X => 1,
                Player::O => 2
            } << (2*i);
        }
    }
    m
}

pub fn maximize(st: State, p: Player) -> Option<(u8, u8)> {
    fn inner(st: State, p: Player) -> (i8, Option<(u8, u8)>) {
        // st caches wins, so this is faster than memo
        if let Some(score) = st.score() {
            return match score {
                Score::Win(w) if w == p => (1, None),
                Score::Win(_) => (-1, None),
                Score::Tie => (0, None)
            };
        }

        let m = densely_pack(st.board(), p);

        if let Some(v) = MEMO.read_sync(&m, |_, &v| v) {
            return v
        }

        assert_eq!(st.turn(), Some(p));

        let v = st.par_succs()
            .map(|(x, y)| {
                let nst = st.do_move(x, y).unwrap();
                let (score, _) = inner(nst, p.other());
                (-score, Some((x, y)))
            })
            .max_by_key(|&(score, _)| score).unwrap();

        let _ = MEMO.insert_sync(m, v);

        v
    }

    inner(st, p).1
}