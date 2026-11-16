//! The ears of the World that the crate writes, through what each gives a host.

#[cfg(unix)]
mod bash;
mod catalog;
#[cfg(unix)]
mod claude;
mod config;
mod disk;
mod kits;
mod offered;
mod operator;
mod provider;
mod reader;
mod schema;
mod search;
mod store;
