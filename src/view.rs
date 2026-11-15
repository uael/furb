//! The meaning of a life that each window shows the same way: the state of each act and of each
//! chain in one set, the desk of the operator, the threads, the spend, the name of the session, and
//! the acts as they stood at a moment. Each event of an act carries its meaning, and the layout of
//! each window stays its own.

mod act;
mod cards;
mod desk;
mod fold;
mod held;
mod ladder;
mod named;
mod page;
mod quote;
mod state;
mod threads;
mod time;
mod words;

pub use act::{Act, Answer, Kind, Maker, Pause, Phase, Read, RunState, Spend, WATCH, operator};
pub use cards::{Card, Part, Sent, Skill, Standing};
pub use desk::{
  Ask, Desk, REFUSED, Tick, choices, declined, desk, hint, takes_nothing, ticks, wants,
};
pub use fold::{Fold, followed, timed};
pub use held::{Held, Threads};
pub use ladder::{Rung, SHOWN, Shown};
#[cfg(test)]
pub use named::word;
pub use named::{NAME, Named, said};
pub use page::{
  Agent, Answered, Block, Context, Exchange, Line, Mark, Output, Page, Portion, ROLLS, Reach,
  Reply, Retrying, Said, Saying, Streamed, Thought, Work, wrote,
};
pub use quote::{Quote, Writing, quotes, writing};
pub use state::{State, chain_state, restate, working};
pub use threads::{
  Aside, Attention, ChainAside, ChainThread, Concern, NoteRun, Origin, Rows, ThreadNode, attention,
  chain_asides, chain_notes, chain_threads, chain_tree, concerns, nest_threads, open,
  order_threads, root_of, thread_of,
};
pub use time::{as_of, moments_of};
pub use words::{age, elapsed, grouped, homed, lines, percent, share, since, size, timer, tokens};
