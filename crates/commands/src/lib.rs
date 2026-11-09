//! Stretchly's command-line grammar (`stretchly pause -d 1h`) and its duration format.

mod commands;

pub use commands::{Command, parse_duration};
