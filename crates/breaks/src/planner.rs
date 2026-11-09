use crate::Settings;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Event {
    MiniBreakNotification,
    LongBreakNotification,
    StartMiniBreak,
    StartLongBreak,
    FinishMiniBreak,
    FinishLongBreak,
    AwaitMiniBreakFinish,
    AwaitLongBreakFinish,
    ResumeBreaks,
}

pub struct BreaksPlanner {
    settings: Settings,
    break_number: i64,
    postpones: u32,
    deadline: Option<i64>,
    delay: i64,
    paused: bool,
    dnd: bool,
    idle_cleared: bool,
    idle_long: bool,
    exclusion: bool,
    suspended: bool,
    next: Event,
}

impl BreaksPlanner {
    pub fn deadline(&self) -> Option<i64> {
        self.deadline
    }

    pub fn new(settings: Settings, now: i64) -> Self {
        let (deadline, next) = if settings.mini_break {
            (
                Some(now + settings.mini_break_interval),
                Event::StartMiniBreak,
            )
        } else if settings.long_break {
            (
                Some(now + settings.mini_break_interval * (settings.long_break_interval + 1)),
                Event::StartLongBreak,
            )
        } else {
            (None, Event::StartMiniBreak)
        };
        let delay = deadline.map_or(0, |deadline| deadline - now);
        let mut planner = Self {
            settings,
            break_number: 1,
            postpones: 0,
            deadline,
            delay,
            paused: false,
            dnd: false,
            idle_cleared: false,
            idle_long: false,
            exclusion: false,
            suspended: false,
            next,
        };
        planner.add_notification();
        planner
    }

    fn add_notification(&mut self) {
        let notification = match self.next {
            Event::StartMiniBreak if self.settings.mini_break_notification => Some((
                Event::MiniBreakNotification,
                self.settings.mini_break_notification_interval,
            )),
            Event::StartLongBreak if self.settings.long_break_notification => Some((
                Event::LongBreakNotification,
                self.settings.long_break_notification_interval,
            )),
            _ => None,
        };
        if let Some((event, interval)) = notification {
            self.deadline = self.deadline.map(|deadline| deadline - interval);
            self.delay -= interval;
            self.next = event;
        }
    }

    pub fn postpone(&mut self, now: i64) -> bool {
        let (delay, limit, notification, offset, event, enabled, percent) = match self.next {
            Event::FinishMiniBreak => (
                self.settings.mini_break_postpone_time,
                self.settings.mini_break_postpones_limit,
                self.settings.mini_break_notification,
                self.settings.mini_break_notification_interval,
                Event::StartMiniBreak,
                self.settings.mini_break_postpone,
                self.settings.mini_break_postponable_duration_percent,
            ),
            Event::FinishLongBreak => (
                self.settings.long_break_postpone_time,
                self.settings.long_break_postpones_limit,
                self.settings.long_break_notification,
                self.settings.long_break_notification_interval,
                Event::StartLongBreak,
                self.settings.long_break_postpone,
                self.settings.long_break_postponable_duration_percent,
            ),
            _ => return false,
        };
        let Some(deadline) = self.deadline else {
            return false;
        };
        let elapsed_percent = (now - (deadline - self.delay)) as f64 / self.delay as f64 * 100.0;
        if !enabled || self.postpones >= limit || elapsed_percent > percent as f64 {
            return false;
        }
        self.postpones += 1;
        self.deadline = Some(now + delay);
        self.delay = delay;
        self.next = event;
        if notification && delay > offset {
            self.add_notification();
        }
        true
    }

    pub fn skip_to_mini(&mut self, now: i64) {
        if self.settings.mini_break
            && self.settings.long_break
            && self.break_number % (self.settings.long_break_interval + 1) == 0
        {
            self.break_number = 1;
        }
        self.next = Event::StartMiniBreak;
        self.deadline = Some(now + 100);
        self.delay = 100;
    }

    pub fn skip_to_long(&mut self, now: i64) {
        if self.settings.mini_break && self.settings.long_break {
            self.break_number = self.settings.long_break_interval + 1;
        }
        self.next = Event::StartLongBreak;
        self.deadline = Some(now + 100);
        self.delay = 100;
    }

    pub fn pause(&mut self, now: i64, duration: i64) {
        self.paused = true;
        self.break_number = 0;
        self.postpones = 0;
        self.next = Event::ResumeBreaks;
        self.deadline = if duration == 1 {
            None
        } else {
            Some(now + duration)
        };
        self.delay = duration;
    }

    pub fn resume(&mut self, now: i64) {
        if self.dnd {
            return;
        }
        *self = Self::new(self.settings.clone(), now);
    }

    pub fn reset(&mut self, now: i64) {
        self.resume(now);
    }

    pub fn focus_assist(&mut self, now: i64, value: i32) {
        if !self.settings.monitor_dnd {
            return;
        }
        let on = value == 1 || value == 2;
        if self.paused {
            self.dnd = on;
            return;
        }
        if on && !self.dnd {
            self.dnd = true;
            if !matches!(self.next, Event::FinishMiniBreak | Event::FinishLongBreak) {
                self.deadline = None;
            }
        } else if !on && self.dnd {
            self.dnd = false;
            if !self.paused
                && !self.exclusion
                && !self.idle_long
                && !matches!(self.next, Event::FinishMiniBreak | Event::FinishLongBreak)
            {
                self.reset(now);
            }
        }
    }

    pub fn idle(&mut self, now: i64, idle_time: i64) {
        if !self.settings.natural_breaks {
            return;
        }
        if idle_time > self.settings.natural_breaks_inactivity_reset_time {
            self.idle_long = true;
        } else if idle_time < 20_000 {
            self.idle_long = false;
        }
        if idle_time > self.settings.natural_breaks_inactivity_reset_time
            && !self.paused
            && !matches!(self.next, Event::FinishMiniBreak | Event::FinishLongBreak)
            && self.deadline.is_some()
        {
            self.idle_cleared = true;
            self.deadline = None;
        } else if idle_time < 20_000 && self.idle_cleared {
            self.idle_cleared = false;
            if !self.paused
                && !self.dnd
                && !self.exclusion
                && !matches!(self.next, Event::FinishMiniBreak | Event::FinishLongBreak)
            {
                self.reset(now);
            }
        }
    }

    pub fn app_exclusion(&mut self, now: i64, active: bool) -> Vec<Event> {
        if self.paused {
            return Vec::new();
        }
        if active && !self.exclusion {
            self.exclusion = true;
            let finished = match self.next {
                Event::FinishMiniBreak => Some(Event::FinishMiniBreak),
                Event::FinishLongBreak => Some(Event::FinishLongBreak),
                _ => None,
            };
            self.deadline = None;
            self.break_number = 0;
            self.postpones = 0;
            finished.into_iter().collect()
        } else if !active && self.exclusion {
            self.exclusion = false;
            if !self.dnd && !self.idle_long {
                self.reset(now);
            }
            Vec::new()
        } else {
            Vec::new()
        }
    }

    pub fn resume_exclusion(&mut self, now: i64, active: bool) {
        if self.paused || matches!(self.next, Event::FinishMiniBreak | Event::FinishLongBreak) {
            return;
        }
        self.app_exclusion(now, !active);
    }

    pub fn processes_changed(&mut self, now: i64, process_names: &[&str]) -> Vec<Event> {
        let Some(rule) = self.settings.app_exclusions.iter().find(|rule| rule.active) else {
            return Vec::new();
        };
        let matched = rule
            .commands
            .iter()
            .any(|command| process_names.iter().any(|name| name.contains(command)));
        if rule.rule == "pause" {
            self.app_exclusion(now, matched)
        } else {
            self.resume_exclusion(now, matched);
            Vec::new()
        }
    }

    pub fn suspend(&mut self, now: i64) {
        if self.settings.pause_for_suspend_or_lock
            && !self.paused
            && !self.dnd
            && !self.idle_cleared
            && !self.exclusion
        {
            self.pause(now, 1);
            self.suspended = true;
        }
    }

    pub fn wake(&mut self, now: i64) {
        if self.suspended {
            self.resume(now);
        }
    }

    pub fn time_to_next_break(&self, now: i64) -> Option<i64> {
        let remaining = self.deadline? - now;
        match self.next {
            Event::StartMiniBreak | Event::StartLongBreak => Some(remaining),
            Event::MiniBreakNotification => {
                Some(remaining + self.settings.mini_break_notification_interval)
            }
            Event::LongBreakNotification => {
                Some(remaining + self.settings.long_break_notification_interval)
            }
            _ => None,
        }
    }

    pub fn progress_percentage(&self, now: i64) -> u32 {
        let offset = match self.next {
            Event::MiniBreakNotification => self.settings.mini_break_notification_interval,
            Event::LongBreakNotification => self.settings.long_break_notification_interval,
            _ => 0,
        };
        let total = self.delay + offset;
        match self.time_to_next_break(now) {
            Some(remaining) if total > 0 => ((1.0 - remaining as f64 / total as f64) * 100.0)
                .round()
                .clamp(0.0, 100.0) as u32,
            _ => 0,
        }
    }

    pub fn finish_break(&mut self, now: i64) -> Option<Event> {
        let event = match self.next {
            Event::FinishMiniBreak => Event::FinishMiniBreak,
            Event::FinishLongBreak => Event::FinishLongBreak,
            _ => return None,
        };
        self.plan_next(now);
        Some(event)
    }

    fn plan_next(&mut self, now: i64) {
        self.postpones = 0;
        self.break_number += 1;
        if !self.settings.mini_break && !self.settings.long_break {
            self.deadline = None;
            return;
        }
        let long = self.settings.long_break
            && (!self.settings.mini_break
                || self.break_number % (self.settings.long_break_interval + 1) == 0);
        let interval = if !self.settings.mini_break {
            self.settings.mini_break_interval * (self.settings.long_break_interval + 1)
        } else {
            self.settings.mini_break_interval
        };
        self.deadline = Some(now + interval);
        self.delay = interval;
        self.next = if long {
            Event::StartLongBreak
        } else {
            Event::StartMiniBreak
        };
        self.add_notification();
        if self.dnd || self.exclusion {
            self.deadline = None;
        }
        if self.idle_long {
            self.idle_cleared = true;
            self.deadline = None;
        }
    }

    pub fn advance(&mut self, now: i64) -> Vec<Event> {
        if !self.deadline.is_some_and(|deadline| now >= deadline) {
            return Vec::new();
        }
        let event = self.next.clone();
        if (event == Event::FinishMiniBreak && self.settings.mini_break_manual_finish)
            || (event == Event::FinishLongBreak && self.settings.long_break_manual_finish)
        {
            self.deadline = None;
            return vec![if event == Event::FinishMiniBreak {
                Event::AwaitMiniBreakFinish
            } else {
                Event::AwaitLongBreakFinish
            }];
        }
        match event {
            Event::ResumeBreaks => {
                self.deadline = None;
                self.resume(now);
            }
            Event::MiniBreakNotification => {
                self.delay = self.settings.mini_break_notification_interval;
                self.deadline = Some(now + self.delay);
                self.next = Event::StartMiniBreak;
            }
            Event::LongBreakNotification => {
                self.delay = self.settings.long_break_notification_interval;
                self.deadline = Some(now + self.delay);
                self.next = Event::StartLongBreak;
            }
            Event::StartMiniBreak => {
                self.delay = self.settings.mini_break_duration;
                self.deadline = Some(now + self.delay);
                self.next = Event::FinishMiniBreak;
            }
            Event::StartLongBreak => {
                self.delay = self.settings.long_break_duration;
                self.deadline = Some(now + self.delay);
                self.next = Event::FinishLongBreak;
            }
            Event::FinishMiniBreak | Event::FinishLongBreak => self.plan_next(now),
            Event::AwaitMiniBreakFinish | Event::AwaitLongBreakFinish => unreachable!(),
        }
        vec![event]
    }
}
