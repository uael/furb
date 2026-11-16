//! The tests of the window, in one program: a headless app of OpenTUI, with a clock that the test
//! moves, on a real daemon. The daemon serves a folder of the test on the runtime of the app, and
//! opens each life on a model that answers from a script.

#[path = "../../../../tests/common/tailscale.rs"]
mod tailscale;
#[path = "../../../../tests/common/yard.rs"]
mod yard;

mod cards;
mod context;
mod daemon;
mod image;
mod input;
mod list;
mod loss;
mod mention;
mod motion;
mod open;
mod palette;
mod part;
mod prose;
mod question;
mod screens;
mod skills;
mod system;
mod text;
mod thread;
mod walk;
