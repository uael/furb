//! The meaning of a life that each window shows the same way: the state of each act and of each
//! chain in one set, the desk of the operator, the threads, the spend, the name of the session, and
//! the acts as they stood at a moment. Each event of an act carries its meaning, and the layout of
//! each window stays its own.

mod cards;
mod desk;
mod fold;
mod held;
mod ladder;
mod named;
mod page;
mod quote;
mod threads;
mod time;

pub use desk::{REFUSED, choices, declined, desk, hint, takes_nothing, ticks, wants};
pub use fold::{Fold, followed, timed};
pub use furb_wire::{
  Act, Agent, Answer, Answered, Aside, Ask, Attention, Block, Card, ChainAside, ChainThread,
  Concern, Context, Desk, Exchange, Kind, Line, Maker, Mark, NoteRun, Origin, Output, Page, Part,
  Pause, Phase, Portion, ROLLS, Reach, Read, Reply, Retrying, RunState, Rung, SHOWN, Said, Saying,
  Sent, Shown, Skill, Spend, Standing, State, Thought, ThreadNode, Threads, Tick, WATCH, Work, age,
  chain_state, elapsed, grouped, homed, lines, operator, percent, restate, share, since, size,
  timer, tokens, working, wrote,
};
pub use held::Held;
#[cfg(test)]
pub use named::word;
pub use named::{NAME, Named, said};
pub use page::Streamed;
pub use quote::{Quote, Writing, quotes, writing};
pub use threads::{
  Rows, attention, chain_asides, chain_notes, chain_threads, chain_tree, concerns, nest_threads,
  open, order_threads, root_of, thread_of,
};
pub use time::{as_of, moments_of};
