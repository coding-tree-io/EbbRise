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
        let active_time_off = !settings.count_active_time_only;
        self.settings = settings;
        let leaving_away =
            self.away && (!self.settings.natural_breaks || self.settings.count_active_time_only);
        if leaving_away {
            self.away = false;
        }
        if active_time_off {
            self.idle_since = None;
            self.release(now, 0);
        }
        if (schedule_changed || leaving_away)
            && !self.in_break()
            && !self.paused
            && !self.dnd()
            && !self.exclusion
        {
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

    #[test]
    fn leaving_classic_away_restarts_unless_another_hold_applies() {
        for active_only in [false, true] {
            let settings = Settings {
                natural_breaks: true,
                count_active_time_only: false,
                ..Settings::default()
            };
            let mut planner = BreaksPlanner::new(settings.clone(), 0);
            planner.idle(1_000_000, 1_000_000);
            assert!(planner.away());
            planner.set_settings(
                Settings {
                    natural_breaks: active_only,
                    count_active_time_only: active_only,
                    ..settings
                },
                1_000_000,
            );
            assert!(!planner.away());
            assert!(planner.deadline().is_some());
        }
        let mut held = BreaksPlanner::new(
            Settings {
                natural_breaks: true,
                monitor_dnd: true,
                ..Settings::default()
            },
            0,
        );
        held.idle(1_000_000, 1_000_000);
        held.focus_assist(1_000_000, 1);
        held.set_settings(
            Settings {
                natural_breaks: false,
                ..held.settings().clone()
            },
            1_000_000,
        );
        assert!(!held.away());
        assert_eq!(held.deadline(), None);
        held.focus_assist(1_001_000, 0);
        assert!(held.deadline().is_some());
    }
}
