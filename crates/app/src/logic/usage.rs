//! The user's use of the PC (`docs/workrave-features.md`): the activity monitor behind "count
//! only active time" and the daily limit, the daily-limit reminder, and the statistics.

use breaks::{Activity, ActivityMonitor};

use super::statistics::{DayStatistics, Statistics};
use super::{BreakScreen, Core, Effect, Environment, Wake};

/// The daily-limit reminder: a break-style window with a still ring.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DailyLimitScreen {
    pub screen: BreakScreen,
    /// The number in the ring, such as "4 h".
    pub number: String,
    /// The word under it.
    pub caption: String,
}

/// What follows the user's use of the PC.
pub(super) struct Usage {
    /// Runs while "count only active time" or the daily limit is on (decision D2).
    monitor: Option<ActivityMonitor>,
    /// Today's active time from before the current monitor: an earlier run, or a monitor
    /// that was turned off.
    carried_ms: i64,
    /// The monitor's active time when today started.
    base_ms: i64,
    /// Today's active time at which the next reminder comes; `None` after "Done for today".
    limit_at: Option<i64>,
    /// The reminder window is open.
    limit_shown: bool,
    last_saved_at: i64,
    last_saved_active: Option<i64>,
    pub(super) stats: Statistics,
}

impl Usage {
    pub(super) fn new(daily_limit_time: i64, now: i64) -> Self {
        Self {
            monitor: None,
            carried_ms: 0,
            base_ms: 0,
            limit_at: Some(daily_limit_time),
            limit_shown: false,
            last_saved_at: now,
            last_saved_active: None,
            stats: Statistics::new("", None),
        }
    }

    /// A new daily-limit time applies unless the user is done for today.
    pub(super) fn set_limit(&mut self, daily_limit_time: i64) {
        if self.limit_at.is_some() {
            self.limit_at = Some(daily_limit_time);
            self.stats.today_mut().daily_limit_postponed_until = None;
        }
    }

    /// Today's active time, ms.
    fn active_today(&self, now: i64) -> i64 {
        let current = self
            .monitor
            .as_ref()
            .map_or(0, |monitor| monitor.active_time(now) - self.base_ms);
        self.carried_ms + current
    }
}

impl Core {
    /// Continues today's record from an earlier run.
    pub fn restore_statistics(&mut self, saved: DayStatistics) {
        self.usage.carried_ms = saved.active_ms.unwrap_or(0);
        self.usage.last_saved_active = saved.active_ms;
        if saved.daily_limit_stopped {
            self.usage.limit_at = None;
        } else if let Some(threshold) = saved.daily_limit_postponed_until {
            self.usage.limit_at = Some(threshold);
        }
        self.usage.stats = Statistics::new(&saved.day.clone(), Some(saved));
    }

    /// Today's record as it is now, with the active time.
    pub fn today_statistics(&self, now: i64) -> DayStatistics {
        let mut today = self.usage.stats.today().clone();
        if self.usage.monitor.is_some() || today.active_ms.is_some() {
            today.active_ms = Some(self.usage.active_today(now));
        }
        today
    }

    /// The records to save: ended days and today, when a count changed since the last call.
    pub fn statistics_to_save(&mut self, now: i64) -> Vec<DayStatistics> {
        let today = self.today_statistics(now);
        if now - self.usage.last_saved_at >= 600_000
            && today.active_ms != self.usage.last_saved_active
        {
            // Marks today changed, so the checkpoint writes its active time.
            self.usage.stats.today_mut();
        }
        let mut records = self.usage.stats.take_changed();
        if let Some(last) = records.last_mut() {
            *last = today;
        }
        records
    }

    pub fn statistics_saved(&mut self, now: i64) {
        self.usage.last_saved_at = now;
        self.usage.last_saved_active = self.today_statistics(now).active_ms;
    }

    /// Updates the day and activity before a final save.
    pub fn prepare_final_statistics(&mut self, now: i64, idle: i64, os: &impl Environment) {
        self.track_usage(now, idle, os);
    }

    /// Gives back records from [`Core::statistics_to_save`] that were not written.
    pub fn keep_unsaved_statistics(&mut self, records: Vec<DayStatistics>) {
        self.usage.stats.keep_unsaved(records);
    }

    /// Reads the activity and rolls over to a new day.
    pub(super) fn track_usage(
        &mut self,
        now: i64,
        idle: i64,
        os: &impl Environment,
    ) -> Vec<Effect> {
        let mut effects = Vec::new();
        // shortcut: activity since the last look stays on the old day, split it if exact boundaries are needed.
        self.observe_activity(now, idle, &mut effects);
        let day = os.day();
        if self.usage.stats.today().day != day {
            let active_ms = self.today_statistics(now).active_ms;
            self.usage.stats.roll_over(&day, active_ms);
            self.usage.carried_ms = 0;
            self.usage.base_ms = self
                .usage
                .monitor
                .as_ref()
                .map_or(0, |m| m.active_time(now));
            self.usage.limit_at = Some(self.settings().daily_limit_time);
            effects.extend(self.close_daily_limit());
        }
        effects
    }

    /// Shows the daily-limit reminder when due, or closes it when the limit was turned off.
    /// Call it after the due break steps, so a break that starts now holds it.
    pub(super) fn check_daily_limit(&mut self, now: i64) -> Option<Effect> {
        if !self.settings().daily_limit {
            return self.close_daily_limit();
        }
        if !self.daily_limit_due(now) {
            return None;
        }
        self.usage.limit_shown = true;
        self.usage.stats.today_mut().daily_limit_shown += 1;
        Some(Effect::ShowDailyLimit(self.daily_limit_screen(now)))
    }

    fn observe_activity(&mut self, now: i64, idle: i64, effects: &mut Vec<Effect>) {
        let s = self.settings();
        if !(s.count_active_time_only || s.daily_limit) {
            if let Some(monitor) = self.usage.monitor.take() {
                self.usage.carried_ms += monitor.active_time(now) - self.usage.base_ms;
                self.usage.base_ms = 0;
            }
            return;
        }
        let change = match &mut self.usage.monitor {
            Some(monitor) => monitor.observe(now, idle),
            None => {
                let monitor = ActivityMonitor::new(now, idle);
                let idle_now = monitor.idle();
                self.usage.monitor = Some(monitor);
                idle_now.then_some(Activity::BecameIdle { since: now })
            }
        };
        for event in change.map_or_else(Vec::new, |change| self.planner.activity(change)) {
            effects.extend(self.carry_out(event, true));
        }
    }

    /// Tells the planner that the user is idle when "count only active time" was just turned on.
    pub(super) fn sync_idle_after_settings(&mut self, now: i64) {
        if self
            .usage
            .monitor
            .as_ref()
            .is_some_and(ActivityMonitor::idle)
        {
            self.planner.activity(Activity::BecameIdle { since: now });
        }
    }

    /// Pause, Do Not Disturb, an app exclusion and a running break hold the reminder.
    fn daily_limit_held(&self) -> bool {
        let planner = &self.planner;
        planner.paused() || planner.dnd() || planner.excluded() || planner.in_break()
    }

    fn daily_limit_due(&self, now: i64) -> bool {
        let usage = &self.usage;
        !usage.limit_shown
            && !self.daily_limit_held()
            && usage
                .limit_at
                .is_some_and(|at| usage.active_today(now) >= at)
    }

    fn daily_limit_screen(&self, now: i64) -> DailyLimitScreen {
        let words = &self.words;
        let hours = self.usage.active_today(now) as f64 / 3_600_000.0;
        let postpone_minutes = self.settings().daily_limit_postpone_time / 60_000;
        DailyLimitScreen {
            screen: BreakScreen {
                kind: "Daily limit".to_owned(),
                headline: "Daily limit reached".to_owned(),
                detail: format!(
                    "You have used the PC for {} today. Time to stop for the day. This notice comes back after {postpone_minutes} more minutes of use.",
                    words.duration(self.usage.active_today(now))
                ),
                source: String::new(),
                postpone: String::new(),
                skip: String::new(),
                // A notice only: acknowledging it is the one thing the window can do.
                finish: "Got it".to_owned(),
            },
            number: format!("{} h", (hours * 10.0).round() / 10.0),
            caption: "today".to_owned(),
        }
    }

    fn close_daily_limit(&mut self) -> Option<Effect> {
        std::mem::take(&mut self.usage.limit_shown).then_some(Effect::CloseDailyLimit)
    }

    /// "Got it" on the daily-limit notice (`postpone` true): it comes back after the postpone time.
    /// `postpone` false stops it for today.
    pub fn on_daily_limit(&mut self, postpone: bool, now: i64) -> Vec<Effect> {
        let Some(close) = self.close_daily_limit() else {
            return Vec::new();
        };
        if postpone {
            let later = self.usage.active_today(now) + self.settings().daily_limit_postpone_time;
            self.usage.limit_at = Some(later);
            self.usage.stats.today_mut().daily_limit_postponed_until = Some(later);
        } else {
            self.usage.limit_at = None;
            let today = self.usage.stats.today_mut();
            today.daily_limit_stopped = true;
            today.daily_limit_postponed_until = None;
        }
        vec![close]
    }

    /// When the activity monitor or the daily limit needs the next look.
    pub(super) fn usage_wake(&self, now: i64, idle: i64) -> Option<Wake> {
        let usage = &self.usage;
        let s = self.settings();
        let Some(monitor) = usage.monitor.as_ref() else {
            // The first look starts the monitor.
            return (s.count_active_time_only || s.daily_limit).then_some(Wake {
                delay: 1_000,
                tolerance: 500,
            });
        };
        let look = monitor.next_check(now, idle);
        // A held reminder waits for the next look, not for a due time already passed.
        let limit_runs =
            s.daily_limit && !usage.limit_shown && !monitor.idle() && !self.daily_limit_held();
        let limit = usage
            .limit_at
            .filter(|_| limit_runs)
            .map(|at| (at - usage.active_today(now)).max(0));
        let delay = limit.map_or(look, |limit| limit.min(look));
        Some(Wake {
            delay,
            tolerance: 500,
        })
    }
}
