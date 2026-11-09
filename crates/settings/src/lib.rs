//! Stretchly's preferences schema and defaults. Reads Stretchly `config.json` files as they are,
//! and saves the port's own config without losing keys it does not know.

mod settings;
mod store;

pub use settings::{AppExclusion, Idea, MorningHour, PauseDuration, Settings};
pub use store::write_atomically;
