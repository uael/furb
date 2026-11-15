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

pub use desk::{
  Ask, Desk, REFUSED, Tick, choices, declined, desk, hint, takes_nothing, ticks, wants,
};
pub use fold::{Fold, followed, timed};
pub use furb_wire::{
  Act, Agent, Answer, Answered, Block, Card, Context, Exchange, Kind, Line, Maker, Mark, Output,
  Page, Part, Pause, Phase, Portion, ROLLS, Reach, Read, Reply, Retrying, RunState, Said, Saying,
  Sent, Skill, Spend, Standing, State, Thought, WATCH, Work, age, chain_state, elapsed, grouped,
  homed, lines, operator, percent, restate, share, since, size, timer, tokens, working, wrote,
};
pub use held::{Held, Threads};
pub use ladder::{Rung, SHOWN, Shown};
#[cfg(test)]
pub use named::word;
pub use named::{NAME, Named, said};
pub use page::Streamed;
pub use quote::{Quote, Writing, quotes, writing};
pub use threads::{
  Aside, Attention, ChainAside, ChainThread, Concern, NoteRun, Origin, Rows, ThreadNode, attention,
  chain_asides, chain_notes, chain_threads, chain_tree, concerns, nest_threads, open,
  order_threads, root_of, thread_of,
};
pub use time::{as_of, moments_of};
