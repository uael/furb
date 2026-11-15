//! What crosses the wire of furb: the meaning of a life as each client shows it, in plain shapes,
//! and the words in which a person reads a time, a size and a count. A client links this crate and
//! no engine and no daemon, so a phone or a web page reads the same meaning as the window.

pub mod act;
pub mod state;
pub mod words;

pub use act::{
  Act, Answer, Kind, Maker, OPERATOR, Pause, Phase, Read, RunState, Spend, WATCH, fault,
  interruption, operator,
};
pub use state::{State, chain_state, restate, working};
pub use words::{age, elapsed, grouped, homed, lines, percent, share, since, size, timer, tokens};
