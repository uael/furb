//! The meaning of a life that each window shows the same way: the state of each act and of each
//! chain in one set, the desk of the operator, the threads, the spend, the name of the session, and
//! the acts as they stood at a moment. Each event of an act carries its meaning, and the layout of
//! each window stays its own.

mod act;
mod desk;
mod fold;
mod held;
mod named;
mod state;
mod threads;
mod time;

pub use act::{Act, Answer, Kind, Pause, Phase, RunState, Spend, operator};
pub use desk::{Ask, Desk, Tick, choices, desk, hint, takes_nothing, ticks, wants};
pub use fold::{Fold, followed};
pub use held::{Held, Threads};
pub use named::{NAME, Named, RENAME, said, word};
pub use state::{State, chain_state, restate, working};
pub use threads::{
  Aside, Attention, ChainAside, ChainThread, Concern, NoteRun, Origin, Rows, ThreadNode, attention,
  chain_asides, chain_notes, chain_threads, chain_tree, concerns, nest_threads, open,
  order_threads, root_of, thread_of,
};
pub use time::{as_of, moments_of};
