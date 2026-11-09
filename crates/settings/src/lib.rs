//! Stretchly's preferences schema and defaults. Reads Stretchly `config.json` files as they are.

mod settings;

pub use settings::{AppExclusion, Idea, MorningHour, PauseDuration, Settings};
