//! Settings changed in the settings window while the app runs.

use super::BreaksPlanner;
use crate::Settings;

/// The settings that decide when breaks come.
fn schedule(s: &Settings) -> [i64; 10] {
    [
        i64::from(s.mini_break),
        i64::from(s.long_break),
        s.mini_break_interval,
        s.mini_break_duration,
        s.long_break_every,
        s.long_break_duration,
        i64::from(s.mini_break_notification),
        i64::from(s.long_break_notification),
        s.mini_break_notification_interval,
        s.long_break_notification_interval,
    ]
}

impl BreaksPlanner {
    /// Applies changed settings. A changed schedule starts a new cycle from `now`, unless a
    /// break runs or breaks are paused; other changes apply from the next step.
    pub fn set_settings(&mut self, settings: Settings, now: i64) {
        let schedule_changed = schedule(&self.settings) != schedule(&settings);
        self.settings = settings;
        if schedule_changed && !self.in_break() && !self.paused {
            self.reset(now);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_new_schedule_restarts_the_cycle_and_other_changes_do_not() {
        let mut planner = BreaksPlanner::new(Settings::default(), 0);
        let first = planner.deadline();
        let louder = Settings {
            volume: 0.2,
            ..Settings::default()
        };
        planner.set_settings(louder, 50_000);
        assert_eq!(planner.deadline(), first);
        let sooner = Settings {
            mini_break_interval: 300_000,
            ..planner.settings().clone()
        };
        planner.set_settings(sooner, 50_000);
        assert_ne!(planner.deadline(), first);
        assert_eq!(planner.settings().volume, 0.2);
    }
}
