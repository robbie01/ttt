use rayon::iter::ParallelIterator as _;

use crate::game::{Player, Score, State};


pub fn maximize(st: State, p: Player) -> (i8, Option<(u8, u8)>) {
    if let Some(score) = st.score() {
        return match score {
            Score::Win(w) if w == p => (1, None),
            Score::Win(_) => (-1, None),
            Score::Tie => (0, None)
        };
    }

    assert_eq!(st.turn(), p);

    st.par_succs()
        .map(|(x, y)| {
            let nst = st.do_move(x, y).unwrap();
            let (score, _) = maximize(nst, p.other());
            (-score, Some((x, y)))
        })
        .max_by_key(|&(score, _)| score).unwrap()
}