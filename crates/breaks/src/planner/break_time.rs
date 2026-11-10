//! What the break window shows about the running break: time left and which buttons it offers.

use super::{BreaksPlanner, Event};
use crate::{can_postpone, can_skip};

/// The running break as the break window sees it. Durations are in ms.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BreakTime {
    /// A Long break, else a Mini break.
    pub long: bool,
    /// Time left; 0 while the break waits for a manual finish.
    pub remaining: i64,
    /// The break's full length.
    pub length: i64,
    /// [`BreaksPlanner::postpone`] would succeed, and strict mode is off.
    pub may_postpone: bool,
    /// The break may be skipped (upstream: not strict, and postponing is not offered).
    pub may_skip: bool,
    /// The time is over and the break waits for [`BreaksPlanner::finish_break`].
    pub awaiting_finish: bool,
}

/// The postpone settings of one break kind.
pub(super) struct PostponeRule {
    pub delay: i64,
    pub limit: u32,
    pub notification_interval: i64,
    pub start: Event,
    pub enabled: bool,
    pub percent: f64,
    pub strict: bool,
}

impl BreaksPlanner {
    /// The running break; `None` outside a break.
    pub fn break_time(&self, now: i64) -> Option<BreakTime> {
        let rule = self.postpone_rule()?;
        let awaiting_finish = self.deadline.is_none();
        let remaining = self.deadline.map_or(0, |deadline| (deadline - now).max(0));
        let passed = self.passed_percent(now);
        // As upstream's `postponable`: the limit is part of it, so Skip shows once it is used.
        let postponable = rule.enabled && self.postpones < rule.limit;
        Some(BreakTime {
            long: rule.start == Event::StartLongBreak,
            remaining,
            length: self.delay,
            may_postpone: !awaiting_finish
                && !rule.strict
                && can_postpone(postponable, passed, rule.percent),
            may_skip: !awaiting_finish && can_skip(rule.strict, postponable, passed, rule.percent),
            awaiting_finish,
        })
    }

    /// The postpone settings for the running break; `None` outside a break.
    pub(super) fn postpone_rule(&self) -> Option<PostponeRule> {
        let s = &self.settings;
        let rule = match self.next {
            Event::FinishMiniBreak => PostponeRule {
                delay: s.mini_break_postpone_time,
                limit: s.mini_break_postpones_limit,
                notification_interval: s.mini_break_notification_interval,
                start: Event::StartMiniBreak,
                enabled: s.mini_break_postpone,
                percent: s.mini_break_postponable_duration_percent as f64,
                strict: s.mini_break_strict_mode,
            },
            Event::FinishLongBreak => PostponeRule {
                delay: s.long_break_postpone_time,
                limit: s.long_break_postpones_limit,
                notification_interval: s.long_break_notification_interval,
                start: Event::StartLongBreak,
                enabled: s.long_break_postpone,
                percent: s.long_break_postponable_duration_percent as f64,
                strict: s.long_break_strict_mode,
            },
            _ => return None,
        };
        Some(rule)
    }

    /// How much of the running break has passed, 0–100; 100 while it waits for a manual finish.
    pub(super) fn passed_percent(&self, now: i64) -> f64 {
        match self.deadline {
            Some(deadline) if self.delay > 0 => {
                (now - (deadline - self.delay)) as f64 / self.delay as f64 * 100.0
            }
            _ => 100.0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Settings;

    fn in_mini_break(extra: &str) -> BreaksPlanner {
        let json = format!(r#"{{"miniBreakNotification":false{extra}}}"#);
        let mut planner = BreaksPlanner::new(Settings::from_json(&json).unwrap(), 0);
        planner.advance(600_000);
        planner
    }

    #[test]
    fn postpone_comes_first_and_skip_follows_it() {
        let planner =
            in_mini_break(r#","miniBreakPostpone":true,"miniBreakPostponableDurationPercent":30"#);
        let early = planner.break_time(603_000).unwrap();
        assert_eq!((early.remaining, early.length), (17_000, 20_000));
        assert!(early.may_postpone && !early.may_skip && !early.long);
        let late = planner.break_time(610_000).unwrap();
        assert!(!late.may_postpone && late.may_skip);
    }

    #[test]
    fn skip_shows_at_once_after_the_postpone_limit_is_used() {
        let mut planner = in_mini_break(r#","miniBreakPostpone":true,"miniBreakPostponesLimit":1"#);
        assert!(planner.postpone(603_000));
        planner.advance(planner.deadline.unwrap());
        let start = planner.deadline.unwrap() - planner.delay;
        let again = planner.break_time(start + 1_000).unwrap();
        assert!(!again.may_postpone && again.may_skip);
    }

    #[test]
    fn strict_mode_offers_neither_button() {
        let planner = in_mini_break(r#","miniBreakStrictMode":true,"miniBreakPostpone":true"#);
        let time = planner.break_time(601_000).unwrap();
        assert!(!time.may_postpone && !time.may_skip);
    }

    #[test]
    fn manual_finish_waits_with_no_time_left() {
        let mut planner = in_mini_break(r#","miniBreakManualFinish":true"#);
        planner.advance(620_000);
        let time = planner.break_time(625_000).unwrap();
        assert!(time.awaiting_finish && time.remaining == 0 && !time.may_skip);
        assert_eq!(
            BreaksPlanner::new(Settings::default(), 0).break_time(0),
            None
        );
    }
}
