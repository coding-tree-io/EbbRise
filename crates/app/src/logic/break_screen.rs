//! What the break window shows: its fixed words, set when the break starts, and the countdown
//! and ring, refreshed once a second from [`BreakTime`].

use breaks::BreakTime;
use i18n::Translator;
use ideas::BreakIdea;

/// The ring's geometry in the window's path coordinates (`ui/break_window.slint`).
const RING_CENTRE: f64 = 118.0;
const RING_RADIUS: f64 = 108.0;

/// The words of a starting break.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BreakScreen {
    /// "Mini break" or "Long break", in the user's language.
    pub kind: String,
    pub headline: String,
    /// The idea's text under a Long break's title; empty for Mini breaks.
    pub detail: String,
    pub postpone: String,
    pub skip: String,
    pub finish: String,
}

/// A button in the break window.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BreakButton {
    Postpone,
    Skip,
    Finish,
}

impl BreakScreen {
    /// Mini ideas have only a text; Long ideas have a title and a text.
    pub(super) fn new(words: &Translator, long: bool, idea: BreakIdea) -> Self {
        let (headline, detail) = if idea.title.is_empty() {
            (idea.text, String::new())
        } else {
            (idea.title, idea.text)
        };
        let kind = if long {
            "main.toLongBreak"
        } else {
            "main.toMiniBreak"
        };
        Self {
            kind: words.text(kind),
            headline,
            detail,
            postpone: words.text("break.postpone"),
            skip: words.text("break.skip"),
            finish: words.text("break.finish"),
        }
    }
}

/// The number in the ring and the break length under it: "7" and "20 seconds",
/// or "3:12" and "5 minutes" once the break is a minute or longer.
pub fn countdown(words: &Translator, time: &BreakTime) -> (String, String) {
    let left = (time.remaining + 999) / 1000;
    let length = time.length / 1000;
    let number = if length >= 60 {
        format!("{}:{:02}", left / 60, left % 60)
    } else {
        left.to_string()
    };
    let caption = if length >= 60 && length % 60 == 0 {
        words.count("utils.minutes", (length / 60) as u64)
    } else {
        words.count("utils.seconds", length as u64)
    };
    (number, caption)
}

/// SVG path commands for the ring's arc: clockwise from the top, as long as the time left.
pub fn ring_commands(time: &BreakTime) -> String {
    if time.length <= 0 || time.remaining <= 0 {
        return String::new();
    }
    let share = (time.remaining as f64 / time.length as f64).min(1.0);
    let (c, r) = (RING_CENTRE, RING_RADIUS);
    let top = c - r;
    if share > 0.9999 {
        // One arc cannot end where it starts, so a full ring is two halves.
        let bottom = c + r;
        return format!("M {c} {top} A {r} {r} 0 1 1 {c} {bottom} A {r} {r} 0 1 1 {c} {top}");
    }
    let angle = share * std::f64::consts::TAU;
    let x = c + r * angle.sin();
    let y = c - r * angle.cos();
    let large = u8::from(share > 0.5);
    format!("M {c} {top} A {r} {r} 0 {large} 1 {x:.1} {y:.1}")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn time(remaining: i64, length: i64) -> BreakTime {
        BreakTime {
            long: false,
            remaining,
            length,
            may_postpone: false,
            may_skip: true,
            awaiting_finish: false,
        }
    }

    #[test]
    fn countdown_shows_seconds_for_short_breaks_and_minutes_for_long_ones() {
        let words = Translator::new("en");
        assert_eq!(
            countdown(&words, &time(6_200, 20_000)),
            ("7".into(), "20 seconds".into())
        );
        assert_eq!(
            countdown(&words, &time(192_000, 300_000)),
            ("3:12".into(), "5 minutes".into())
        );
        assert_eq!(countdown(&words, &time(0, 90_000)).1, "90 seconds");
    }

    #[test]
    fn ring_follows_the_time_left_clockwise_from_the_top() {
        assert_eq!(
            ring_commands(&time(5_000, 20_000)),
            "M 118 10 A 108 108 0 0 1 226.0 118.0"
        );
        assert_eq!(
            ring_commands(&time(15_000, 20_000)),
            "M 118 10 A 108 108 0 1 1 10.0 118.0"
        );
        assert!(ring_commands(&time(20_000, 20_000)).contains("118 226 A"));
        assert_eq!(ring_commands(&time(0, 20_000)), "");
    }

    #[test]
    fn long_ideas_split_into_headline_and_detail() {
        let words = Translator::new("en");
        let idea = |title: &str, text: &str| BreakIdea {
            title: title.into(),
            text: text.into(),
        };
        let long = BreakScreen::new(&words, true, idea("Walk", "Stand up and walk."));
        assert_eq!(
            (long.kind.as_str(), long.headline.as_str()),
            ("Long break", "Walk")
        );
        assert_eq!(long.detail, "Stand up and walk.");
        let mini = BreakScreen::new(&words, false, idea("", "Look far away."));
        assert_eq!(
            (mini.headline.as_str(), mini.detail.as_str()),
            ("Look far away.", "")
        );
        assert_eq!(mini.skip, "Skip this break");
    }
}
