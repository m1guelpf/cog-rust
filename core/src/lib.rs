#![warn(clippy::all, clippy::pedantic, clippy::nursery)]

pub mod http;
mod spec;

pub use spec::{Cog, CogResponse};
