//! The tray tooltip's status line, as upstream's `statusMessages.js`.

use breaks::{BreaksPlanner, Event};
use i18n::Translator;

/// What the planner is doing, such as "Next Mini break in about 9 minutes"; empty while a
/// break runs or while nothing is planned.
pub(super) fn tray_message(planner: &BreaksPlanner, words: &Translator, now: i64) -> String {
    let paused = words.text("statusMessages.paused");
    if planner.in_break() {
        return String::new();
    }
    if planner.paused() {
        return match planner.deadline() {
            Some(deadline) => format!(
                "{paused} - {} {}",
                words.text("statusMessages.resuming"),
                time_in(words, deadline - now)
            ),
            None => format!("{paused} {}", words.text("statusMessages.indefinitely")),
        };
    }
    if planner.dnd() {
        return format!("{paused} - {}", words.text("statusMessages.dndMode"));
    }
    if planner.excluded() {
        return format!("{paused} - {}", words.text("statusMessages.appExclusion"));
    }
    let Some(remaining) = planner.time_to_next_break(now) else {
        return String::new();
    };
    let long = matches!(
        planner.next(),
        Event::StartLongBreak | Event::LongBreakNotification
    );
    if long {
        return format!(
            "{} {}",
            words.text("statusMessages.nextLongBreak"),
            time_in(words, remaining)
        );
    }
    let mut message = format!(
        "{} {}",
        words.text("statusMessages.nextMiniBreak"),
        time_in(words, remaining)
    );
    if let Some(long) = planner.time_to_long_break(now) {
        message += &format!(
            "\n{} {}",
            words.text("statusMessages.nextLongBreak"),
            time_in(words, long)
        );
    }
    message
}

/// "in about 9 minutes" (upstream `formatTimeIn`).
fn time_in(words: &Translator, milliseconds: i64) -> String {
    words.with("utils.inAbout", &[("count", &words.duration(milliseconds))])
}
