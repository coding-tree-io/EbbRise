//! Translations for the texts the backend shows: tray, tooltip, notifications and break
//! ideas. It reads Stretchly's own i18next locale files.

mod languages;
mod locales;
mod translator;

pub use languages::LANGUAGES;
pub use translator::Translator;
