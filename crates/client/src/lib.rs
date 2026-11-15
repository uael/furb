//! The client of furb with no drawing: what each machine told a client, and in what order its
//! cards stand. It does no input and no output: a client gives it each frame that a machine sends,
//! so a window, a phone or a test reads the machines the same way.

mod machine;

pub use machine::{Client, Heard, Key, Link, Listing, Told};
