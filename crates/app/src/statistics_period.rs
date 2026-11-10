//! The Statistics window's periods and words: the Day, Week and Month ranges, the chart
//! bars (design C2) and the shoreline streak (design P2).

use jiff::ToSpan;
use jiff::civil::Date;

use crate::logic::{BreakCounts, DayStatistics};
use crate::ui::ChartBar;

/// Days on the shoreline, today included.
pub(crate) const SHORE_DAYS: i64 = 14;

/// What the window shows: the switch at its top.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Range {
    Day,
    /// Seven days that end on the anchor.
    Week,
    /// The calendar month of the anchor.
    Month,
}

impl Range {
    pub(crate) fn from_index(index: i32) -> Self {
        match index {
            1 => Self::Week,
            2 => Self::Month,
            _ => Self::Day,
        }
    }

    pub(crate) fn index(self) -> i32 {
        self as i32
    }

    /// The first and last day shown for `anchor`.
    pub(crate) fn period(self, anchor: Date) -> (Date, Date) {
        match self {
            Self::Day => (anchor, anchor),
            Self::Week => (anchor.saturating_sub(6.days()), anchor),
            Self::Month => (anchor.first_of_month(), anchor.last_of_month()),
        }
    }

    /// The anchor one period before (`step` -1) or after (`step` 1).
    pub(crate) fn step(self, anchor: Date, step: i64) -> Date {
        match self {
            Self::Day => anchor.saturating_add(step.days()),
            Self::Week => anchor.saturating_add((7 * step).days()),
            Self::Month => anchor.saturating_add(step.months()),
        }
    }
}

/// Breaks taken (completed or natural) and breaks skipped on `day`.
pub(crate) fn taken_and_skipped(day: &DayStatistics) -> (u32, u32) {
    let taken = |c: &BreakCounts| c.completed + c.natural;
    (
        taken(&day.mini) + taken(&day.long),
        day.mini.skipped + day.long.skipped,
    )
}

/// A day on the shoreline.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Shore {
    /// No break came.
    Empty = 0,
    /// Breaks came and none was skipped.
    Shell = 1,
    /// One or more breaks were skipped.
    Pebble = 2,
}

pub(crate) fn shore_state(day: &DayStatistics) -> Shore {
    match taken_and_skipped(day) {
        (_, 1..) => Shore::Pebble,
        (1.., 0) => Shore::Shell,
        _ => Shore::Empty,
    }
}

/// Days with no skipped break in a row, back from today. A day with no breaks, such as a
/// weekend away, neither adds to the streak nor ends it.
pub(crate) fn streak(days: &[DayStatistics]) -> usize {
    days.iter()
        .rev()
        .map(shore_state)
        .take_while(|state| *state != Shore::Pebble)
        .filter(|state| *state == Shore::Shell)
        .count()
}

pub(crate) fn chart_bar(range: Range, day: &DayStatistics, index: usize, count: usize) -> ChartBar {
    let hours = day.active_ms.unwrap_or(0) as f32 / 3_600_000.0;
    let (taken, skipped) = taken_and_skipped(day);
    let share = if taken + skipped == 0 {
        -1.0
    } else {
        taken as f32 / (taken + skipped) as f32
    };
    let date: Option<Date> = day.day.parse().ok();
    let label = match (range, date) {
        (Range::Week, Some(date)) => date.strftime("%a").to_string(),
        (Range::Month, Some(date)) if index.is_multiple_of(5) || index + 1 == count => {
            date.strftime("%-d %b").to_string()
        }
        _ => String::new(),
    };
    let breaks = match taken + skipped {
        0 => "no breaks".to_owned(),
        all => format!("{taken} of {all} breaks taken"),
    };
    ChartBar {
        hours,
        share,
        label: label.into(),
        description: format!(
            "{}: {}, {breaks}",
            day_label(&day.day, false),
            active_time(day.active_ms)
        )
        .into(),
    }
}

/// The top of the chart's scale: whole hours, even so the middle line is whole too, at least 2.
pub(crate) fn top_hours(most: f32) -> f32 {
    let hours = most.ceil().max(2.0) as u32;
    hours.next_multiple_of(2) as f32
}

/// "4 – 10 Nov", "28 Oct – 3 Nov" or "November 2026".
pub(crate) fn period_label(range: Range, first: Date, last: Date) -> String {
    match range {
        Range::Month => first.strftime("%B %Y").to_string(),
        _ if first.month() == last.month() => {
            format!("{} – {}", first.day(), last.strftime("%-d %b"))
        }
        _ => format!("{} – {}", first.strftime("%-d %b"), last.strftime("%-d %b")),
    }
}

/// "M", "T", "W"… for `day`.
pub(crate) fn weekday_letter(day: &str) -> String {
    day.parse::<Date>()
        .map(|date| date.strftime("%a").to_string().chars().take(1).collect())
        .unwrap_or_default()
}

/// "Today", or the date such as "Sun 8 Nov 2026".
pub(crate) fn day_label(day: &str, today: bool) -> String {
    if today {
        return "Today".to_owned();
    }
    day.parse::<Date>()
        .map(|date| date.strftime("%a %-d %b %Y").to_string())
        .unwrap_or_else(|_| day.to_owned())
}

/// "5 h 12 min", or "Not measured" when neither active-time feature was on.
pub(crate) fn active_time(active_ms: Option<i64>) -> String {
    let Some(ms) = active_ms else {
        return "Not measured".to_owned();
    };
    let minutes = ms / 60_000;
    match (minutes / 60, minutes % 60) {
        (0, minutes) => format!("{minutes} min"),
        (hours, minutes) => format!("{hours} h {minutes} min"),
    }
}

/// "14 of 18": completed breaks of the breaks shown.
pub(crate) fn completed(day: &DayStatistics) -> String {
    let shown = day.mini.shown + day.long.shown;
    let done = day.mini.completed + day.long.completed;
    format!("{done} of {shown}")
}

/// The daily-limit line; empty when no reminder came.
pub(crate) fn daily_limit(day: &DayStatistics) -> String {
    match (day.daily_limit_shown, day.daily_limit_stopped) {
        (0, _) => String::new(),
        (1, false) => "Daily limit: 1 reminder.".to_owned(),
        (1, true) => "Daily limit: 1 reminder; you stopped for the day.".to_owned(),
        (n, false) => format!("Daily limit: {n} reminders."),
        (n, true) => format!("Daily limit: {n} reminders; you stopped for the day."),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn labels_read_naturally() {
        assert_eq!(day_label("2026-11-08", false), "Sun 8 Nov 2026");
        assert_eq!(day_label("2026-11-09", true), "Today");
        assert_eq!(active_time(None), "Not measured");
        assert_eq!(active_time(Some(18_720_000)), "5 h 12 min");
        assert_eq!(active_time(Some(59_000)), "0 min");
        let mut day = DayStatistics::default();
        day.mini.shown = 15;
        day.mini.completed = 12;
        day.long.shown = 3;
        day.long.completed = 2;
        assert_eq!(completed(&day), "14 of 18");
        assert_eq!(daily_limit(&day), "");
        day.daily_limit_shown = 2;
        day.daily_limit_stopped = true;
        assert_eq!(
            daily_limit(&day),
            "Daily limit: 2 reminders; you stopped for the day."
        );
        let date = |s: &str| s.parse::<Date>().unwrap();
        let (first, last) = Range::Week.period(date("2026-11-03"));
        assert_eq!(period_label(Range::Week, first, last), "28 Oct – 3 Nov");
        let (first, last) = Range::Month.period(date("2026-11-10"));
        assert_eq!((first, last), (date("2026-11-01"), date("2026-11-30")));
        assert_eq!(period_label(Range::Month, first, last), "November 2026");
        assert_eq!(
            Range::Month.step(date("2026-03-31"), -1),
            date("2026-02-28")
        );
        assert_eq!(top_hours(7.2), 8.0);
        assert_eq!(top_hours(0.0), 2.0);
    }

    #[test]
    fn a_skip_ends_the_streak_and_a_day_without_breaks_does_not() {
        let day = |taken: u32, skipped: u32| {
            let mut day = DayStatistics::default();
            day.mini.completed = taken;
            day.mini.skipped = skipped;
            day
        };
        let days = [
            day(5, 0),
            day(4, 1),
            day(6, 0),
            day(0, 0),
            day(3, 0),
            day(2, 0),
        ];
        assert_eq!(streak(&days), 3);
        assert_eq!(streak(&[day(5, 0), day(1, 1)]), 0);
        assert_eq!(shore_state(&day(0, 0)), Shore::Empty);
    }
}
