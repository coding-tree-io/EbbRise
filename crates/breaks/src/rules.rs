pub fn can_postpone(postpone: bool, passed_percent: f64, postpone_percent: f64) -> bool {
    postpone && passed_percent <= postpone_percent
}

pub fn can_skip(strict: bool, postpone: bool, passed_percent: f64, postpone_percent: f64) -> bool {
    !strict && !can_postpone(postpone, passed_percent, postpone_percent)
}

pub fn exclusion_blocks(rule: &str, commands: &[&str], process_names: &[&str]) -> bool {
    let found = commands
        .iter()
        .any(|command| process_names.iter().any(|name| name.contains(command)));
    if rule == "pause" {
        found
    } else {
        !found
    }
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

pub fn allowed_audio_filename(name: &str) -> bool {
    allowed_filename(name, &["wav"])
}
pub fn allowed_image_filename(name: &str) -> bool {
    allowed_filename(name, &["png", "jpg", "jpeg", "webp", "gif"])
}

pub fn tray_icon_for_minutes(minutes: i64) -> String {
    if (0..=99).contains(&minutes) {
        format!("trayNumber{minutes}.ico")
    } else {
        "tray.ico".into()
    }
}

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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Rect {
    pub x: i64,
    pub y: i64,
    pub width: i64,
    pub height: i64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Display {
    pub id: i64,
    pub bounds: Rect,
}

pub fn target_display<'a>(
    displays: &'a [Display],
    primary: i64,
    cursor: i64,
    all_screens: bool,
    setting: &str,
    index: i64,
) -> Option<&'a Display> {
    let cursor_display = || displays.iter().find(|display| display.id == cursor);
    if !all_screens {
        match setting {
            "primary" => return displays.iter().find(|display| display.id == primary),
            "cursor" => return cursor_display(),
            _ => {
                return setting
                    .parse::<usize>()
                    .ok()
                    .and_then(|i| displays.get(i))
                    .or_else(cursor_display)
            }
        }
    }
    usize::try_from(index)
        .ok()
        .and_then(|i| displays.get(i))
        .or_else(cursor_display)
}

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

pub fn locale_has_backend_keys(locale: &str, fallback: &str) -> bool {
    let (Ok(locale), Ok(fallback)) = (
        serde_json::from_str::<serde_json::Value>(locale),
        serde_json::from_str::<serde_json::Value>(fallback),
    ) else {
        return false;
    };
    let keys = [
        "toolTipHeader",
        "miniBreakIn",
        "longBreakIn",
        "resumingBreaks",
        "downloadLatestVersion",
        "toLongBreak",
        "toMiniBreak",
        "skipToTheNext",
        "resume",
        "pause",
        "untilMorning",
        "indefinitely",
        "resetBreaks",
        "quitStretchly",
        "preferences",
    ];
    let status = [
        "paused",
        "indefinitely",
        "dndMode",
        "appExclusion",
        "nextLongBreak",
        "nextMiniBreak",
        "afterMiniBreak_one",
        "afterMiniBreak_other",
        "resuming",
    ];
    let utils = ["remaining", "inAbout", "seconds_one", "seconds_other"];
    [
        ("main", &keys[..]),
        ("statusMessages", &status[..]),
        ("utils", &utils[..]),
    ]
    .iter()
    .all(|(section, keys)| {
        keys.iter().all(|key| {
            locale
                .get(section)
                .and_then(|value| value.get(key))
                .or_else(|| fallback.get(section).and_then(|value| value.get(key)))
                .is_some_and(|value| value.is_string())
        })
    })
}
