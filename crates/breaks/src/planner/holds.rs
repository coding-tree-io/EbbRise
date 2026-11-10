//! Reactions to outside signals that hold or restart breaks: Do Not Disturb, natural breaks
//! (idle), app exclusions, and suspend or lock. Upstream keeps these in its managers' event handlers.

use super::{BreaksPlanner, Event};
use crate::exclusion_blocks;

/// Idle time below which the user counts as back (upstream `naturalBreaksManager.js`).
const BACK_FROM_IDLE_MS: i64 = 20_000;
const FOCUS_ASSIST_PRIORITY_ONLY: i32 = 1;
const FOCUS_ASSIST_ALARMS_ONLY: i32 = 2;

impl BreaksPlanner {
    /// Applies a new Focus Assist value: 1 or 2 hold breaks; 0 and -1 (unsupported) release them.
    ///
    /// A running break continues. Ignored unless `monitor_dnd` is on.
    pub fn focus_assist(&mut self, now: i64, value: i32) {
        let on = matches!(value, FOCUS_ASSIST_PRIORITY_ONLY | FOCUS_ASSIST_ALARMS_ONLY);
        if !self.settings.monitor_dnd || on == self.dnd {
            return;
        }
        self.dnd = on;
        self.update_hold(now, on);
    }

    /// Holds due breaks for a full-screen application or presentation.
    pub fn full_screen(&mut self, now: i64, on: bool) {
        if on == self.full_screen {
            return;
        }
        self.full_screen = on;
        if on {
            self.update_hold(now, true);
        } else if !self.paused
            && !self.in_break()
            && !self.dnd()
            && !self.exclusion
            && !self.away
            && !self.held()
            && self.deadline.is_none()
            && (self.settings.mini_break || self.settings.long_break)
        {
            self.deadline = Some(now);
        }
    }

    fn update_hold(&mut self, now: i64, on: bool) {
        if self.paused || self.in_break() {
            return;
        }
        if on {
            self.deadline = None;
        } else if !self.dnd() && !self.exclusion && !self.away {
            self.reset(now);
        }
    }

    /// Applies the time since the last input, in ms (natural breaks).
    ///
    /// Idle past `natural_breaks_inactivity_reset_time` holds breaks; input again restarts the
    /// cycle. Returns the `*TakenNaturally` events when the user leaves. Off while only active
    /// time counts, which has its own natural breaks ([`Self::activity`]).
    pub fn idle(&mut self, now: i64, idle_time: i64) -> Vec<Event> {
        let s = &self.settings;
        if !s.natural_breaks || s.count_active_time_only {
            return Vec::new();
        }
        let mut taken = Vec::new();
        if idle_time > s.natural_breaks_inactivity_reset_time {
            let waiting = !self.away && !self.paused && !self.in_break() && self.deadline.is_some();
            if waiting && s.mini_break {
                taken.push(Event::MiniBreakTakenNaturally);
            }
            if waiting && s.long_break && self.long_due.is_some() {
                taken.push(Event::LongBreakTakenNaturally);
            }
            self.away = true;
            if !self.paused && !self.in_break() {
                self.deadline = None;
            }
        } else if idle_time < BACK_FROM_IDLE_MS && self.away {
            self.away = false;
            if !self.paused && !self.dnd() && !self.exclusion && !self.in_break() {
                self.reset(now);
            }
        }
        taken
    }

    /// Applies the running process names to the first active app exclusion rule.
    ///
    /// Returns the `Finish*` event when a "pause" rule ends a running break.
    pub fn processes_changed(&mut self, now: i64, process_names: &[&str]) -> Vec<Event> {
        let Some(rule) = self.settings.app_exclusions.iter().find(|rule| rule.active) else {
            return Vec::new();
        };
        let pause_rule = rule.rule == "pause";
        let blocked = exclusion_blocks(&rule.rule, &rule.commands, process_names);
        // Upstream ends a running break only for a "pause" rule.
        if !pause_rule && self.in_break() {
            return Vec::new();
        }
        self.app_exclusion(now, blocked)
    }

    fn app_exclusion(&mut self, now: i64, blocked: bool) -> Vec<Event> {
        if self.paused || blocked == self.exclusion {
            return Vec::new();
        }
        self.exclusion = blocked;
        if blocked {
            let finished = self.in_break().then(|| self.next.clone());
            // Upstream `clear()` drops the pending step, so the ended break no longer counts as running.
            self.next = Event::StartMiniBreak;
            self.deadline = None;
            self.postpones = 0;
            finished.into_iter().collect()
        } else {
            if !self.dnd() && !self.away && !self.in_break() {
                self.reset(now);
            }
            Vec::new()
        }
    }

    /// The PC sleeps or locks: pauses breaks if `pause_for_suspend_or_lock` is on and
    /// nothing else holds them.
    pub fn suspend(&mut self, now: i64) {
        if self.settings.pause_for_suspend_or_lock
            && !self.paused
            && !self.dnd()
            && !self.away
            && !self.exclusion
        {
            self.pause(now, Self::INDEFINITELY);
            self.suspended = true;
        }
    }

    /// The PC wakes or unlocks: resumes breaks if [`Self::suspend`] paused them.
    pub fn wake(&mut self, now: i64) {
        if self.suspended {
            self.resume(now);
        }
    }
}
