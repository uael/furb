//! The tests of the window, in one program: a headless app of OpenTUI, with a clock that the test
//! moves, on a real daemon. The daemon serves a folder of the test on the runtime of the app, and
//! opens each life on a model that answers from a script.

#[path = "../../../host/tests/suite/counted.rs"]
mod counted;
#[path = "../../../../tests/common/tailscale.rs"]
mod tailscale;
#[path = "../../../../tests/common/yard.rs"]
mod yard;

mod cards;
mod commands;
mod context;
mod copy;
mod daemon;
mod extremes;
mod here;
mod image;
mod input;
mod keys;
mod list;
mod loss;
mod memory;
mod mention;
mod motion;
mod open;
mod palette;
mod part;
mod prose;
mod question;
mod screens;
mod settings;
mod skills;
mod system;
mod text;
mod thread;
mod views;
mod voice;
mod walk;

/// The allocator that counts each byte that the process holds and the calls of each thread, which
/// the test of the memory of a window reads.
#[global_allocator]
static COUNTED: here::Here = here::Here;
