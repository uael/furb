//! The meaning of a life that each window shows the same way: the state of each act and of each
//! chain in one set, the desk of the operator, the threads, the spend, and the acts as they stood
//! at a moment. Each event of an act carries its meaning, and the layout of each window stays its
//! own.

mod act;
mod desk;
mod fold;
mod held;
mod state;
mod threads;
mod time;

pub use act::{Act, Answer, Kind, Pause, Phase, RunState, Spend, operator};
pub use desk::{Ask, Desk, Tick, choices, desk, hint, takes_nothing, ticks, wants};
pub use fold::{Fold, followed};
pub use held::{Held, Threads};
pub use state::{State, chain_state, restate, working};
pub use threads::{
  Aside, Attention, ChainAside, ChainThread, Concern, Origin, Rows, ThreadNode, attention,
  chain_asides, chain_threads, concerns, nest_threads, open, order_threads, root_of, thread_of,
};
pub use time::{as_of, moments_of};
