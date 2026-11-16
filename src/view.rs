//! The meaning of a life that each window shows the same way: the state of each act and of each
//! chain in one set, the desk of the operator, the threads, the spend, the name of the session, and
//! the acts as they stood at a moment. Each event of an act carries its meaning, and the layout of
//! each window stays its own.

use std::collections::BTreeMap;

mod cards;
mod desk;
mod fold;
mod held;
mod ladder;
mod named;
mod page;
mod quote;

pub(crate) use quote::closes;
mod threads;
mod time;

pub use desk::{REFUSED, choices, declined, desk, hint, takes_nothing, ticks, wants};
pub use fold::{Fold, followed, timed};
pub use furb_wire::{
  Act, Agent, Answer, Answered, Aside, Ask, Attention, Block, Card, ChainAside, ChainThread,
  Concern, Context, Desk, Exchange, Kind, LITERAL, Line, Made, Maker, Mark, NoteRun, Origin,
  Output, Page, Part, Pause, Phase, Portion, ROLLS, Reach, Read, Reply, Retrying, RunState, Rung,
  SHOWN, Said, Saying, Sent, Shown, Skill, Spend, Standing, State, Thought, ThreadNode, Threads,
  Tick, WATCH, Work, Written, age, chain_state, elapsed, grouped, homed, lines, operator, percent,
  reason, restate, share, since, size, timer, tokens, working, wrote,
};
pub use held::{Held, Reader};
pub use ladder::Asked;
pub use named::{NAME, Named, said};
pub use page::Streamed;
pub use quote::{Quote, Writing, quotes, writing};
pub use threads::{
  Rows, attention, chain_asides, chain_notes, chain_threads, chain_tree, concerns, nest_threads,
  open, order_threads, root_of, thread_of,
};
pub use time::{as_of, moments_of};

/// The line of the word that made each act while a word stood on the stack, counted from 1, by the
/// name of the act. The sandbox finds it as each act is made, at each open of the record too, so it
/// is a function of the record, which keeps no line: [`crate::Engine::lines`] gives it, and
/// [`Streamed`] holds it for a page.
pub type Lines = BTreeMap<String, u32>;
