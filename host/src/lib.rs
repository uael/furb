//! The host of furb: each life on a thread of its own, which a [`LifeHandle`] reaches from any
//! thread.
//!
//! [`Life::spawn`] opens a life on a thread that owns its engine, since the engine is not `Send`,
//! with a runtime of tokio of its own. A handle says a [`Command`] to the life and takes its
//! [`Reply`], and reads the [`Event`]s of the life in order, each under its [`Seq`], from any point
//! of its log. The host gives each life the same ears, so each life of a machine opens the same
//! way, whatever client asked for it.

mod ears;
mod error;
mod event;
mod life;

pub use error::LifeError;
pub use event::{Act, Event, Seq, State};
pub use life::{Command, Life, LifeHandle, Reply};
