//! What crosses the wire of furb: the meaning of a life as each client shows it, in plain shapes,
//! and the words in which a person reads a time, a size and a count. A client links this crate and
//! no engine and no daemon, so a phone or a web page reads the same meaning as the window.

pub mod act;
pub mod anchor;
pub mod card;
pub mod config;
pub mod desk;
pub mod error;
pub mod event;
pub mod ladder;
mod older;
pub mod open;
pub mod page;
pub mod protocol;
pub mod state;
pub mod threads;
pub mod words;

pub use act::{
  Act, Answer, Kind, Maker, OPERATOR, Pause, Phase, Read, RunState, Spend, WATCH, fault,
  interruption, operator, reason,
};
pub use card::{Card, Part, Sent, Skill, Standing};
pub use config::Layer;
pub use desk::{Ask, Desk, Tick};
pub use error::ProtocolError;
pub use event::{Event, FollowUp, Seq};
pub use ladder::{Rung, SHOWN, Shown};
pub use open::Stage;
pub use page::{
  Agent, Answered, Block, Context, Exchange, LITERAL, Line, Made, Mark, Output, Page, Portion,
  ROLLS, Reach, Reply, Retrying, Said, Saying, Thought, Work, Written, stands, wrote,
};
pub use state::{State, chain_state, restate, working};
pub use threads::{
  Aside, Attention, Began, ChainAside, ChainThread, ChainTrack, Concern, NoteRun, Origin,
  ThreadNode, Threads,
};
pub use words::{age, elapsed, grouped, homed, lines, percent, share, since, size, timer, tokens};
