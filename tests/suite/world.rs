//! The ears of the World that the crate writes, through what each gives a host.

#[cfg(unix)]
mod bash;
mod catalog;
#[cfg(unix)]
mod claude;
mod codex;
mod config;
mod disk;
mod files;
mod images;
mod kits;
mod local;
mod log;
mod offered;
mod operator;
mod provider;
mod reader;
mod schema;
mod scratch;
mod search;
mod store;
