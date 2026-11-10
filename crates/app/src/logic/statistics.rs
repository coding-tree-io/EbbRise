//! Per-day break statistics (`docs/workrave-features.md`, feature 3): what happened to each
//! break, the active time and the daily-limit reminders. A day starts at 04:00 local time.

use breaks::Event;
use serde::{Deserialize, Serialize};

/// What happened to the breaks of one kind on one day.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct BreakCounts {
    /// Break windows opened.
    pub shown: u32,
    /// Breaks that ran to their end, or that the user finished.
    pub completed: u32,
    /// Breaks that time away replaced.
    pub natural: u32,
    /// Breaks skipped, or ended by an app exclusion.
    pub skipped: u32,
    /// Postpone presses.
    pub postponed: u32,
}

/// One day's record in `statistics.json`.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct DayStatistics {
    /// The day as `YYYY-MM-DD`; it starts at 04:00.
    pub day: String,
    pub mini: BreakCounts,
    pub long: BreakCounts,
    /// Active time in ms; `None` when no activity was measured that day.
    pub active_ms: Option<i64>,
    /// Daily-limit reminders shown.
    pub daily_limit_shown: u32,
    /// The user chose "Done for today".
    pub daily_limit_stopped: bool,
}

/// Returns each day in the inclusive range, with empty records for missing days.
pub fn day_range(
    records: &[DayStatistics],
    first: jiff::civil::Date,
    last: jiff::civil::Date,
) -> Vec<DayStatistics> {
    let mut days = Vec::new();
    let mut day = first;
    while day <= last {
        let key = day.to_string();
        days.push(
            records
                .iter()
                .find(|record| record.day == key)
                .cloned()
                .unwrap_or_else(|| DayStatistics {
                    day: key,
                    ..DayStatistics::default()
                }),
        );
        if day == last {
            break;
        }
        let Ok(next) = day.checked_add(jiff::Span::new().days(1)) else {
            break;
        };
        day = next;
    }
    days
}

/// Today's record, and the records that wait to be saved.
#[derive(Debug)]
pub struct Statistics {
    today: DayStatistics,
    /// Days that ended and were not saved yet.
    ended: Vec<DayStatistics>,
    changed: bool,
}

impl Statistics {
    /// Starts `day` from `saved`, its record from an earlier run, when there is one.
    pub fn new(day: &str, saved: Option<DayStatistics>) -> Self {
        let today = saved
            .filter(|record| record.day == day)
            .unwrap_or_else(|| DayStatistics {
                day: day.to_owned(),
                ..DayStatistics::default()
            });
        Self {
            today,
            ended: Vec::new(),
            changed: false,
        }
    }

    pub fn today(&self) -> &DayStatistics {
        &self.today
    }

    pub(super) fn today_mut(&mut self) -> &mut DayStatistics {
        self.changed = true;
        &mut self.today
    }

    /// Ends today's record with its final `active_ms` and starts one for `day`, when `day` is
    /// not today's day; returns whether it did.
    pub fn roll_over(&mut self, day: &str, active_ms: Option<i64>) -> bool {
        if self.today.day == day {
            return false;
        }
        self.today.active_ms = active_ms;
        let ended = std::mem::replace(
            &mut self.today,
            DayStatistics {
                day: day.to_owned(),
                ..DayStatistics::default()
            },
        );
        // The empty day is the start-up record before the first look at the clock.
        if !ended.day.is_empty() {
            self.ended.push(ended);
        }
        self.changed = true;
        true
    }

    /// Counts a planner event. `on_time` is false when something other than the break's own
    /// time ended it: Skip, or an app exclusion.
    pub fn record(&mut self, event: &Event, on_time: bool) {
        let (long, count): (bool, fn(&mut BreakCounts)) = match event {
            Event::StartMiniBreak => (false, |c| c.shown += 1),
            Event::StartLongBreak => (true, |c| c.shown += 1),
            Event::FinishMiniBreak if on_time => (false, |c| c.completed += 1),
            Event::FinishLongBreak if on_time => (true, |c| c.completed += 1),
            Event::FinishMiniBreak => (false, |c| c.skipped += 1),
            Event::FinishLongBreak => (true, |c| c.skipped += 1),
            Event::MiniBreakTakenNaturally => (false, |c| c.natural += 1),
            Event::LongBreakTakenNaturally => (true, |c| c.natural += 1),
            _ => return,
        };
        let today = self.today_mut();
        count(if long {
            &mut today.long
        } else {
            &mut today.mini
        });
    }

    pub fn postponed(&mut self, long: bool) {
        let today = self.today_mut();
        let counts = if long {
            &mut today.long
        } else {
            &mut today.mini
        };
        counts.postponed += 1;
    }

    /// The records to save since the last call: ended days and today. Empty when nothing changed.
    pub fn take_changed(&mut self) -> Vec<DayStatistics> {
        if !std::mem::take(&mut self.changed) {
            return Vec::new();
        }
        let mut records = std::mem::take(&mut self.ended);
        records.push(self.today.clone());
        records
    }

    /// Gives back records from [`Self::take_changed`] that were not saved: the ended days wait
    /// for the next save, and today is saved again from its live record.
    pub fn keep_unsaved(&mut self, mut records: Vec<DayStatistics>) {
        records.retain(|record| record.day != self.today.day);
        records.append(&mut self.ended);
        self.ended = records;
        self.changed = true;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn range_fills_missing_calendar_days() {
        let saved = DayStatistics {
            day: "2024-02-29".into(),
            active_ms: Some(123),
            ..DayStatistics::default()
        };
        let days = day_range(
            std::slice::from_ref(&saved),
            "2024-02-28".parse().unwrap(),
            "2024-03-01".parse().unwrap(),
        );
        assert_eq!(
            days.iter().map(|d| d.day.as_str()).collect::<Vec<_>>(),
            ["2024-02-28", "2024-02-29", "2024-03-01"]
        );
        assert_eq!(days[1], saved);
        assert_eq!(days[0].active_ms, None);
    }

    #[test]
    fn events_count_into_the_kind_of_their_break() {
        let mut stats = Statistics::new("2026-11-09", None);
        stats.record(&Event::StartMiniBreak, true);
        stats.record(&Event::FinishMiniBreak, true);
        stats.record(&Event::StartLongBreak, true);
        stats.record(&Event::FinishLongBreak, false);
        stats.record(&Event::MiniBreakTakenNaturally, true);
        stats.record(&Event::MiniBreakNotification, true);
        stats.postponed(false);
        let today = stats.today();
        assert_eq!(
            today.mini,
            BreakCounts {
                shown: 1,
                completed: 1,
                natural: 1,
                skipped: 0,
                postponed: 1
            }
        );
        assert_eq!((today.long.shown, today.long.skipped), (1, 1));
    }

    #[test]
    fn a_new_day_starts_a_new_record_and_both_are_saved_once() {
        let saved = DayStatistics {
            day: "2026-11-09".into(),
            active_ms: Some(60_000),
            ..DayStatistics::default()
        };
        let mut stats = Statistics::new("2026-11-09", Some(saved.clone()));
        assert_eq!(stats.today(), &saved, "a restart keeps today's record");
        assert!(stats.take_changed().is_empty());
        stats.record(&Event::StartMiniBreak, true);
        assert!(stats.roll_over("2026-11-10", Some(90_000)));
        let records = stats.take_changed();
        let days: Vec<_> = records.iter().map(|record| record.day.as_str()).collect();
        assert_eq!(days, ["2026-11-09", "2026-11-10"]);
        assert_eq!(records[0].mini.shown, 1);
        assert_eq!(
            records[0].active_ms,
            Some(90_000),
            "the day ends with its live total"
        );
        assert!(stats.take_changed().is_empty());
        stats.keep_unsaved(records);
        let days: Vec<_> = stats.take_changed().into_iter().map(|r| r.day).collect();
        assert_eq!(days, ["2026-11-09", "2026-11-10"], "an unsaved day waits");
        assert!(!stats.roll_over("2026-11-10", None));
    }
}
