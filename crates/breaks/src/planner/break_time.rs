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
    /// The countdown is held while the user interacts.
    pub activity_paused: bool,
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
    /// Holds a running countdown during input until the user has been still for 2 seconds.
    /// Returns whether the scheduled deadline changed.
    pub fn break_activity(&mut self, now: i64, idle: i64) -> bool {
        let Some(deadline) = self.deadline.filter(|_| self.in_break()) else {
            return false;
        };
        let idle = idle.max(0);
        if idle < 2_000 && self.break_held_since.is_none() {
            let since = now - idle;
            if since < deadline - self.delay || since >= deadline {
                return false;
            }
            self.break_held_since = Some(since);
            return true;
        } else if idle >= 2_000
            && let Some(since) = self.break_held_since.take()
        {
            let resume = (now - idle + 2_000).clamp(since, now);
            self.deadline = Some(deadline + resume - since);
            return true;
        }
        false
    }

    /// The running break; `None` outside a break.
    pub fn break_time(&self, now: i64) -> Option<BreakTime> {
        let rule = self.postpone_rule()?;
        let awaiting_finish = self.deadline.is_none();
        let now = self.break_held_since.unwrap_or(now);
        let remaining = self.deadline.map_or(0, |deadline| (deadline - now).max(0));
        let passed = self.passed_percent(now);
        // As upstream's `postponable`: the limit is part of it, so Skip shows once it is used.
        let postponable = rule.enabled && self.postpones < rule.limit;
        Some(BreakTime {
            long: rule.start == Event::StartLongBreak,
            remaining,
            activity_paused: self.break_held_since.is_some(),
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
        let now = self.break_held_since.unwrap_or(now);
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

    #[test]
    fn input_holds_the_countdown_and_postpone_limit_until_two_seconds_still() {
        let mut planner =
            in_mini_break(r#","miniBreakPostpone":true,"miniBreakPostponableDurationPercent":30"#);
        assert!(planner.break_activity(604_000, 500));
        assert_eq!(planner.deadline(), None);
        let time = planner.break_time(620_000).unwrap();
        assert_eq!(time.remaining, 16_500);
        assert!(time.activity_paused && time.may_postpone && !time.may_skip);
        assert!(planner.advance(620_000).is_empty());
        assert!(!planner.break_activity(620_000, 1_999));
        assert!(planner.break_activity(620_001, 2_000));
        assert_eq!(planner.deadline(), Some(636_501));
        let time = planner.break_time(620_001).unwrap();
        assert_eq!(time.remaining, 16_500);
        assert!(!time.activity_paused && time.may_postpone);
        assert!(planner.advance(636_500).is_empty());
        assert_eq!(planner.advance(636_501), vec![Event::FinishMiniBreak]);
    }

    #[test]
    fn delayed_activity_check_resumes_at_two_seconds_still() {
        let mut planner = in_mini_break("");
        assert!(planner.break_activity(604_000, 0));
        assert!(planner.break_activity(610_000, 4_000));
        assert_eq!(planner.deadline(), Some(624_000));
        assert_eq!(planner.break_time(610_000).unwrap().remaining, 14_000);
    }

    #[test]
    fn input_outside_the_break_does_not_hold_it() {
        let mut planner = in_mini_break("");
        assert!(!planner.break_activity(601_000, 1_500));
        assert_eq!(planner.deadline(), Some(620_000));
        assert!(!planner.break_activity(620_001, 0));
        assert_eq!(planner.advance(620_001), vec![Event::FinishMiniBreak]);
    }

    #[test]
    fn activity_does_not_affect_manual_finish_wait() {
        let mut planner = in_mini_break(r#","miniBreakManualFinish":true"#);
        assert_eq!(planner.advance(620_000), vec![Event::AwaitMiniBreakFinish]);
        assert!(!planner.break_activity(621_000, 0));
        let time = planner.break_time(621_000).unwrap();
        assert!(time.awaiting_finish && !time.activity_paused);
        assert_eq!(planner.finish_break(621_000), Some(Event::FinishMiniBreak));
    }

    #[test]
    fn ending_a_held_break_clears_activity_state() {
        for end in 0..5 {
            let mut planner = in_mini_break(r#","miniBreakPostpone":true"#);
            assert!(planner.break_activity(601_000, 0));
            match end {
                0 => assert_eq!(planner.finish_break(602_000), Some(Event::FinishMiniBreak)),
                1 => assert!(planner.postpone(602_000)),
                2 => planner.skip_to_long(602_000, None),
                3 => planner.reset(602_000),
                _ => planner.pause(602_000, 10_000),
            }
            assert!(planner.deadline().is_some(), "end method {end}");
            assert!(!planner.break_activity(602_000, 0));
        }
    }

    #[test]
    fn exclusion_clears_activity_state() {
        let mut planner = in_mini_break(
            r#","appExclusions":[{"rule":"pause","active":true,"commands":["meeting"]}]"#,
        );
        assert!(planner.break_activity(601_000, 0));
        assert_eq!(
            planner.processes_changed(602_000, &["meeting.exe"]),
            vec![Event::FinishMiniBreak]
        );
        planner.processes_changed(603_000, &[]);
        assert!(planner.deadline().is_some());
    }
}
