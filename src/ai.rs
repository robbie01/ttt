use std::hash::{BuildHasherDefault, DefaultHasher};

use rayon::iter::ParallelIterator as _;
use scc::HashMap;

use crate::{game::{Player, Score, State}, N};

static MEMO: HashMap<([Option<Player>; (N*N) as usize], Player), (i8, Option<(u8, u8)>), BuildHasherDefault<DefaultHasher>> = HashMap::with_hasher(BuildHasherDefault::new());

pub fn maximize(st: State, p: Player) -> (i8, Option<(u8, u8)>) {
    if let Some(v) = MEMO.read_sync(&(st.board(), p), |_, &v| v) {
        return v
    }

    if let Some(score) = st.score() {
        return match score {
            Score::Win(w) if w == p => (1, None),
            Score::Win(_) => (-1, None),
            Score::Tie => (0, None)
        };
    }

    assert_eq!(st.turn(), p);

    let v = st.par_succs()
        .map(|(x, y)| {
            let nst = st.do_move(x, y).unwrap();
            let (score, _) = maximize(nst, p.other());
            (-score, Some((x, y)))
        })
        .max_by_key(|&(score, _)| score).unwrap();

    let _ = MEMO.insert_sync((st.board(), p), v);

    v
}