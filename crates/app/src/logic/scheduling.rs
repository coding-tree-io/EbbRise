//! Scheduling checks and the next wake-up.

use super::{Core, Effect, Environment, Event, Wake};

pub(super) const AWAY_CHECK_MS: i64 = 2_000;
const FULL_SCREEN_CHECK_MS: i64 = 10_000;

impl Core {
    pub(super) fn check_full_screen(&mut self, now: i64, os: &impl Environment) {
        let due = self.planner.deadline().is_some_and(|at| at <= now)
            && matches!(
                self.planner.next(),
                Event::MiniBreakNotification
                    | Event::LongBreakNotification
                    | Event::StartMiniBreak
                    | Event::StartLongBreak
            );
        if !self.settings().no_breaks_in_full_screen {
            self.full_screen_check_at = None;
            self.planner.full_screen(now, false);
        } else if self.full_screen_check_at.is_some_and(|at| at <= now)
            || (self.full_screen_check_at.is_none() && due)
        {
            let busy = os
                .full_screen_busy()
                .unwrap_or(self.full_screen_check_at.is_some());
            self.planner.full_screen(now, busy);
            self.full_screen_check_at = busy.then_some(now + FULL_SCREEN_CHECK_MS);
        }
    }

    /// Several steps can be due at once, for example after a long busy period.
    pub(super) fn run_due_steps(&mut self, now: i64) -> Vec<Effect> {
        let mut effects = Vec::new();
        while !(self.full_screen_check_at.is_some()
            && matches!(
                self.planner.next(),
                Event::MiniBreakNotification
                    | Event::LongBreakNotification
                    | Event::StartMiniBreak
                    | Event::StartLongBreak
            ))
            && self
                .planner
                .deadline()
                .is_some_and(|deadline| deadline <= now)
        {
            for event in self.planner.advance(now) {
                effects.extend(self.carry_out(event, true));
            }
        }
        effects
    }

    /// The one timer the app needs: the earliest of the break deadline, the idle check and
    /// the app exclusion check. On a tie the break deadline wins, with its tighter tolerance.
    pub fn next_wake(&self, now: i64, idle: i64) -> Option<Wake> {
        let settings = self.settings();
        let held_start = self.full_screen_check_at.is_some()
            && matches!(
                self.planner.next(),
                Event::MiniBreakNotification
                    | Event::LongBreakNotification
                    | Event::StartMiniBreak
                    | Event::StartLongBreak
            );
        let break_due = self
            .planner
            .deadline()
            .filter(|_| !held_start)
            .map(|deadline| Wake {
                delay: deadline - now,
                tolerance: 10,
            });
        let classic_natural_breaks = settings.natural_breaks && !settings.count_active_time_only;
        let idle_check = classic_natural_breaks.then(|| {
            let delay = if self.planner.away() {
                AWAY_CHECK_MS
            } else {
                let earliest_reset_crossing = settings.natural_breaks_inactivity_reset_time - idle;
                earliest_reset_crossing.max(1_000)
            };
            Wake {
                delay,
                tolerance: 1_000,
            }
        });
        let exclusion_check = self.app_exclusion_check_due().map(|due| Wake {
            delay: (due - now).max(0),
            tolerance: 1_000,
        });
        let usage = self.usage_wake(now, idle);
        let full_screen_check = self.full_screen_check_at.map(|at| Wake {
            delay: (at - now).max(0),
            tolerance: 1_000,
        });
        [
            break_due,
            idle_check,
            exclusion_check,
            usage,
            full_screen_check,
        ]
        .into_iter()
        .flatten()
        .min_by_key(|wake| (wake.delay, wake.tolerance))
    }
}
