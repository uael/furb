//! The doors of the crate: one per host language, each behind the feature that names it.
//!
//! A door hands the engine to a host that is no rust: the host's ears cross as one object heard by name, the
//! verbs of the contract are said by name with their words, and every value crosses as monty carries it, made
//! into what that language holds. Nothing of the engine lives in a door.

#[cfg(feature = "python")]
pub mod py;

#[cfg(feature = "typescript")]
pub mod ts;
