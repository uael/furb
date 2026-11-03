//! The errors of a life that the host holds.

use thiserror::Error;

/// Why a life gave no reply to a command.
#[derive(Debug, Error)]
pub enum LifeError {
  /// The life refused the command, with what the engine raised.
  #[error("{0}")]
  Refused(String),
  /// The life ended before it replied, so it replies to no command more.
  #[error("the life ended")]
  Ended,
  /// The machine gave the life no thread.
  #[error("the life has no thread: {0}")]
  Thread(#[source] std::io::Error),
}

impl From<furb::Fault> for LifeError {
  fn from(fault: furb::Fault) -> Self {
    LifeError::Refused(fault.to_string())
  }
}
