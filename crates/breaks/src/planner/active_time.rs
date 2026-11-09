//! Count only active time (`countActiveTimeOnly`): idle time holds the wait for a break, and
//! idle time as long as a break counts as that break (`docs/workrave-features.md`, feature 1).

use super::{BreaksPlanner, Event};
use crate::Activity;

impl BreaksPlanner {
    /// The wait for the next break is held: the user is idle while only active time counts.
    pub fn held(&self) -> bool {
        self.held_since.is_some()
    }

    /// Applies an activity change; ignored unless only active time counts.
    ///
    /// Returns the `*TakenNaturally` events for the breaks that the idle time replaced.
    pub fn activity(&mut self, change: Activity) -> Vec<Event> {
        if !self.settings.count_active_time_only {
            return Vec::new();
        }
        match change {
            Activity::BecameIdle { since } => {
                self.idle_since = Some(since);
                self.hold(since);
                Vec::new()
            }
            Activity::Returned { at } => {
                let idle_time = self.idle_since.take().map_or(0, |since| at - since);
                self.release(at, idle_time)
            }
        }
    }

    /// Holds a waiting step from `since`, but not from before the wait started. A postponed
    /// break is not held: postpone times stay clock time.
    fn hold(&mut self, since: i64) {
        let waiting =
            self.deadline.is_some() && !self.paused && !self.in_break() && self.postpones == 0;
        if self.held_since.is_none() && waiting {
            self.held_since = Some(since.max(self.wait_start));
        }
    }

    /// After a new wait starts: holds it at once when the user is already idle.
    pub(super) fn hold_if_idle(&mut self, now: i64) {
        if self.settings.count_active_time_only && self.idle_since.is_some() {
            self.hold(now);
        }
    }

    /// Ends a hold at `at`: the wait moves on by the held time. With natural breaks on, idle
    /// time (`idle_time`) at least as long as a break restarts that break's wait.
    pub(super) fn release(&mut self, at: i64, idle_time: i64) -> Vec<Event> {
        let Some(held) = self.held_since.take() else {
            return Vec::new();
        };
        let shift = (at - held).max(0);
        self.deadline = self.deadline.map(|deadline| deadline + shift);
        self.long_due = self.long_due.map(|due| due + shift);
        let s = &self.settings;
        if !s.natural_breaks {
            return Vec::new();
        }
        let mini_waiting = matches!(
            self.next,
            Event::StartMiniBreak | Event::MiniBreakNotification
        );
        let mini = s.mini_break && mini_waiting && idle_time >= s.mini_break_duration;
        let long = s.long_break && self.long_due.is_some() && idle_time >= s.long_break_duration;
        let mut taken = Vec::new();
        if mini {
            taken.push(Event::MiniBreakTakenNaturally);
        }
        if long {
            self.long_due = None;
            taken.push(Event::LongBreakTakenNaturally);
        }
        if mini || long {
            self.plan_next(at);
        }
        taken
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Settings;

    fn active_time(natural: bool) -> Settings {
        Settings {
            count_active_time_only: true,
            natural_breaks: natural,
            mini_break_notification: false,
            long_break_notification: false,
            mini_break_interval: 600_000,
            mini_break_duration: 20_000,
            long_break_every: 3_600_000,
            long_break_duration: 300_000,
            ..Settings::default()
        }
    }

    fn idle(since: i64) -> Activity {
        Activity::BecameIdle { since }
    }

    #[test]
    fn idle_time_holds_the_wait_and_moves_it_on() {
        let mut planner = BreaksPlanner::new(active_time(false), 0);
        assert_eq!(planner.deadline(), Some(600_000));
        planner.activity(idle(100_000));
        assert!(planner.held());
        assert_eq!(planner.deadline(), None);
        assert_eq!(planner.time_to_next_break(400_000), Some(500_000));
        let taken = planner.activity(Activity::Returned { at: 400_000 });
        assert!(taken.is_empty(), "natural breaks are off");
        assert_eq!(planner.deadline(), Some(900_000));
        assert_eq!(planner.time_to_long_break(400_000), Some(3_500_000));
    }

    #[test]
    fn idle_as_long_as_a_break_counts_as_that_break() {
        let mut planner = BreaksPlanner::new(active_time(true), 0);
        // 5 s idle: shorter than the Mini break, so the wait only moves on.
        planner.activity(idle(100_000));
        let taken = planner.activity(Activity::Returned { at: 105_000 });
        assert!(taken.is_empty());
        assert_eq!(planner.deadline(), Some(605_000));

        // 25 s idle: a Mini break, so its wait starts again; the Long wait moves on.
        planner.activity(idle(200_000));
        let taken = planner.activity(Activity::Returned { at: 225_000 });
        assert_eq!(taken, vec![Event::MiniBreakTakenNaturally]);
        assert_eq!(planner.deadline(), Some(825_000));
        assert_eq!(planner.time_to_long_break(225_000), Some(3_405_000));

        // Almost 6 min idle: both waits start again.
        planner.activity(idle(300_000));
        let taken = planner.activity(Activity::Returned { at: 655_000 });
        assert_eq!(
            taken,
            vec![
                Event::MiniBreakTakenNaturally,
                Event::LongBreakTakenNaturally
            ]
        );
        assert_eq!(planner.deadline(), Some(1_255_000));
        assert_eq!(planner.time_to_long_break(655_000), Some(3_600_000));
    }

    #[test]
    fn a_wait_that_starts_while_idle_is_held_at_once() {
        let mut planner = BreaksPlanner::new(active_time(false), 0);
        planner.activity(idle(100_000));
        planner.skip_to_mini(150_000, None);
        assert!(!planner.held(), "a break the user asks for still comes");
        planner.advance(150_100);
        assert!(planner.in_break());
        planner.advance(170_100);
        assert!(planner.held(), "the next wait starts held");
        assert_eq!(planner.time_to_next_break(500_000), Some(600_000));
    }

    #[test]
    fn a_postponed_break_keeps_clock_time() {
        let mut planner = BreaksPlanner::new(
            Settings {
                mini_break_postpone: true,
                mini_break_postpones_limit: 1,
                ..active_time(false)
            },
            0,
        );
        planner.advance(600_000);
        assert!(planner.postpone(601_000));
        planner.activity(idle(605_000));
        assert!(!planner.held());
    }

    #[test]
    fn turning_the_setting_off_ends_the_hold() {
        let mut planner = BreaksPlanner::new(active_time(false), 0);
        planner.activity(idle(100_000));
        planner.set_settings(
            Settings {
                count_active_time_only: false,
                ..active_time(false)
            },
            300_000,
        );
        assert!(!planner.held());
        assert_eq!(planner.deadline(), Some(800_000));
    }
}
