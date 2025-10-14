use std::hash::{BuildHasherDefault, DefaultHasher};

use rayon::iter::ParallelIterator as _;
use scc::{hash_map::Entry, HashMap};

use crate::{game::{Player, Score, State}, N};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Bound {
    Exact,
    Lower,
    Upper
}

static MEMO: HashMap<u64, (i8, Option<(u8, u8)>, Bound), BuildHasherDefault<DefaultHasher>> = HashMap::with_hasher(BuildHasherDefault::new());

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
    fn inner(st: State, p: Player, par_depth: u8, mut alpha: i8, beta: i8) -> (i8, Option<(u8, u8)>) {
        // st caches wins, so this is faster than memo
        if let Some(score) = st.score() {
            return match score {
                Score::Win(w) if w == p => (1, None),
                Score::Win(_) => (-1, None),
                Score::Tie => (0, None)
            };
        }

        let m = densely_pack(st.board(), p);

        if let Some((score, pos, bound)) = MEMO.read_sync(&m, |_, &v| v) &&
            (bound == Bound::Exact || (bound == Bound::Lower && score >= beta) || (bound == Bound::Upper && score <= alpha)) {
            return (score, pos)
        }

        assert_eq!(st.turn(), Some(p));

        if par_depth == 0 {
            let old_alpha = alpha;

            let mut max = None;
            for (x, y) in st.succs() {
                let nst = st.do_move(x, y).unwrap();
                let (score, _) = inner(nst, p.other(), 0, -beta, -alpha);
                let score = -score;
                if max.is_none_or(|(ms, _)| score > ms) {
                    max = Some((score, Some((x, y))));
                }
                alpha = alpha.max(score);
                if alpha >= beta { break }
            }

            let (score, pos) = max.unwrap();

            let ent = MEMO.entry_sync(m);
            if !matches!(ent, Entry::Occupied(ref o) if o.2 == Bound::Exact) {
                let bound = if score <= old_alpha {
                    Bound::Upper
                } else if score >= beta {
                    Bound::Lower
                } else {
                    Bound::Exact
                };
                ent.insert_entry((score, pos, bound));
            }
            (score, pos)
        } else {
            let (score, pos) = st.par_succs()
                .map(|(x, y)| {
                    let nst = st.do_move(x, y).unwrap();
                    let (score, _) = inner(nst, p.other(), par_depth - 1, -beta, -alpha);
                    (-score, Some((x, y)))
                })
                .max_by_key(|&(score, _)| score).unwrap();
            
            MEMO.upsert_sync(m, (score, pos, Bound::Exact));

            (score, pos)
        }
    }

    inner(st, p, 2, -2, 2).1
}