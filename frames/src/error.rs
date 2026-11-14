use std::{fmt, io, path::PathBuf};

/// What stops a command of the frames.
#[derive(Debug, thiserror::Error)]
pub(crate) enum Error {
  #[error("{} cannot be read: {source}", path.display())]
  Read { path: PathBuf, source: io::Error },
  #[error("{} cannot be written: {source}", path.display())]
  Write { path: PathBuf, source: io::Error },
  #[error("the recording is not valid: {0}")]
  Cast(&'static str),
  #[error("a frame or a recording is not valid JSON: {0}")]
  Json(#[from] serde_json::Error),
  #[error("a text cannot be written: {0}")]
  Text(#[from] fmt::Error),
  #[error("the image of a frame cannot be made: {0}")]
  Png(#[from] png::EncodingError),
  #[error("{0:?} is not a color #rrggbb")]
  Color(String),
  #[error("{what} cannot run: {source}")]
  Process { what: &'static str, source: io::Error },
  #[error("`tmux {command}` failed: {said}")]
  Tmux { command: String, said: String },
  #[error("the daemon of the capture did not say that it listens")]
  Listen,
  #[error("the step {step} waited for {until:?}, and the screen shows:\n{screen}")]
  Step { step: String, until: String, screen: String },
  #[error("{} holds no {program}: run `cargo build -p furb-window -p furb-cli` first", folder.display())]
  Missing { folder: PathBuf, program: &'static str },
  #[error("no scenario is named {0:?}")]
  Scenario(String),
  #[error("{0} of the captures failed")]
  Failed(usize),
}
