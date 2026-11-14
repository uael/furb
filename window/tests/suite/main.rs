//! The tests of the window, in one program: a headless app of OpenTUI, with a clock that the test
//! moves, on a real daemon. The daemon serves a folder of the test on the runtime of the app, and
//! opens each life on a model that answers from a script.

#[path = "../../../tests/common/yard.rs"]
mod yard;

mod cards;
mod daemon;
mod input;
mod screens;
mod thread;
