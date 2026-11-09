use crate::Settings;

/// Whether a break may be postponed: postponing is on and at most `postpone_percent` of
/// the break has passed. Does not check the postpone limit, as upstream.
pub fn can_postpone(postpone: bool, passed_percent: f64, postpone_percent: f64) -> bool {
    postpone && passed_percent <= postpone_percent
}

/// Whether a break may be skipped: not strict, and postponing is not offered instead.
pub fn can_skip(strict: bool, postpone: bool, passed_percent: f64, postpone_percent: f64) -> bool {
    !strict && !can_postpone(postpone, passed_percent, postpone_percent)
}

/// Whether an app exclusion rule holds breaks: a `"pause"` rule while one of its
/// `commands` runs, a `"resume"` rule while none does. Names match by substring, as upstream.
pub fn exclusion_blocks(rule: &str, commands: &[String], process_names: &[&str]) -> bool {
    let found = commands.iter().any(|command| {
        process_names
            .iter()
            .any(|name| name.contains(command.as_str()))
    });
    if rule == "pause" { found } else { !found }
}

fn allowed_filename(name: &str, extensions: &[&str]) -> bool {
    let Some((stem, extension)) = name.rsplit_once('.') else {
        return false;
    };
    !stem.is_empty()
        && stem
            .bytes()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, b'.' | b'_' | b'-'))
        && extensions
            .iter()
            .any(|ext| extension.eq_ignore_ascii_case(ext))
}

/// Whether `name` is a safe custom sound file name: letters, digits, `._-`, ending `.wav`.
pub fn allowed_audio_filename(name: &str) -> bool {
    allowed_filename(name, &["wav"])
}

/// Whether `name` is a safe custom image file name: letters, digits, `._-`, ending in
/// `.png`, `.jpg`, `.jpeg`, `.webp` or `.gif`.
pub fn allowed_image_filename(name: &str) -> bool {
    allowed_filename(name, &["png", "jpg", "jpeg", "webp", "gif"])
}

/// Tray icon file for the "time" style: the minutes left (0–99), else the plain icon.
pub fn tray_icon_for_minutes(minutes: i64) -> String {
    if (0..=99).contains(&minutes) {
        format!("trayNumber{minutes}.ico")
    } else {
        "tray.ico".into()
    }
}

/// English "1 minute 58 seconds remaining" text, rounded to whole seconds.
///
/// shortcut: English only; upstream uses `humanize-duration` per locale (docs/test-port-skips.md).
pub fn format_time_remaining(milliseconds: i64) -> String {
    let mut seconds = ((milliseconds.unsigned_abs() as f64) / 1000.0).round() as u64;
    let mut parts = Vec::new();
    for (unit, size) in [
        ("day", 86_400),
        ("hour", 3_600),
        ("minute", 60),
        ("second", 1),
    ] {
        let value = seconds / size;
        seconds %= size;
        if value > 0 {
            parts.push(format!(
                "{value} {unit}{}",
                if value == 1 { "" } else { "s" }
            ));
        }
    }
    if parts.is_empty() {
        parts.push("0 seconds".into());
    }
    format!("{} remaining", parts.join(" "))
}

/// A screen rectangle in pixels.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Rect {
    /// Left edge.
    pub x: i64,
    /// Top edge.
    pub y: i64,
    /// Width.
    pub width: i64,
    /// Height.
    pub height: i64,
}

/// A connected display.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Display {
    /// OS display id.
    pub id: i64,
    /// Position and size.
    pub bounds: Rect,
}

/// The display for a break window (upstream `displayManager.getTargetDisplay`).
///
/// Without `all_screens`, follows `screen`; with it, `index` picks the display. An invalid
/// index falls back to the display with the cursor. `None` only if the given ids are unknown.
pub fn target_display<'a>(
    displays: &'a [Display],
    primary: i64,
    cursor: i64,
    settings: &Settings,
    index: i64,
) -> Option<&'a Display> {
    let cursor_display = || displays.iter().find(|display| display.id == cursor);
    if !settings.all_screens {
        let setting = settings.screen.as_str();
        match setting {
            "primary" => return displays.iter().find(|display| display.id == primary),
            "cursor" => return cursor_display(),
            _ => {
                return setting
                    .parse::<usize>()
                    .ok()
                    .and_then(|i| displays.get(i))
                    .or_else(cursor_display);
            }
        }
    }
    usize::try_from(index)
        .ok()
        .and_then(|i| displays.get(i))
        .or_else(cursor_display)
}

/// Break window rectangle on a display: the whole display when `fullscreen`, else centred.
pub fn window_position(bounds: Rect, width: i64, height: i64, fullscreen: bool) -> Rect {
    if fullscreen {
        bounds
    } else {
        Rect {
            x: bounds.x + (bounds.width - width).div_euclid(2),
            y: bounds.y + (bounds.height - height).div_euclid(2),
            width,
            height,
        }
    }
}
