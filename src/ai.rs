use rayon::iter::ParallelIterator as _;

use crate::game::{Player, Score, State};


pub fn maximize(st: State, p: Player) -> (i8, (u32, u32)) {
    if let Some(score) = st.score() {
        return match score {
            Score::Win(w) if w == p => (1, (255, 255)),
            Score::Win(_) => (-1, (255, 255)),
            Score::Tie => (0, (255, 255))
        };
    }

    assert_eq!(st.turn(), p);

    st.succs()
        .map(|(x, y)| {
            let nst = st.do_move(x, y).unwrap();
            let (score, _) = maximize(nst, p.other());
            (-score, (x, y))
        })
        .max_by_key(|&(score, _)| score).unwrap()
}