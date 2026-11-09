//! The Slint windows, compiled from `ui/main.slint` by the build script.
// The generated layout code trips this lint; it is not ours to split.
#![allow(clippy::cognitive_complexity)]

slint::include_modules!();

/// The palette mode for `themeSource`: 0 follows Windows, 1 is light, 2 is dark.
pub(crate) fn theme(settings: &settings::Settings) -> i32 {
    match settings.theme_source.as_str() {
        "light" => 1,
        "dark" => 2,
        _ => 0,
    }
}
