use crate::{Settings, can_postpone};

mod active_time;
mod break_time;
mod holds;
mod settings_change;

pub use break_time::BreakTime;

/// Upstream `skipToMiniBreak (delay = 100)`.
const SKIP_DELAY_MS: i64 = 100;

/// A step the planner took; the caller carries it out (toast, break window, sound).
///
/// The same names mark the pending step (upstream's scheduler `reference`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Event {
    /// Warn that a Mini break comes in `mini_break_notification_interval`.
    MiniBreakNotification,
    /// Warn that a Long break comes in `long_break_notification_interval`.
    LongBreakNotification,
    /// A Mini break starts now.
    StartMiniBreak,
    /// A Long break starts now.
    StartLongBreak,
    /// The Mini break ended; the next break is planned.
    FinishMiniBreak,
    /// The Long break ended; the next break is planned.
    FinishLongBreak,
    /// The Mini break time is over; wait for [`BreaksPlanner::finish_break`] (manual finish).
    AwaitMiniBreakFinish,
    /// The Long break time is over; wait for [`BreaksPlanner::finish_break`] (manual finish).
    AwaitLongBreakFinish,
    /// A timed pause ended and breaks restarted.
    ResumeBreaks,
    /// Idle time replaced the next Mini break (natural break); never a pending step.
    MiniBreakTakenNaturally,
    /// Idle time replaced the next Long break (natural break); never a pending step.
    LongBreakTakenNaturally,
}

/// Stretchly's break schedule as a sans-IO state machine (upstream `breaksPlanner.js`).
///
/// Every method takes `now`, a monotonic time in ms; the caller wakes it at [`Self::deadline`]
/// and calls [`Self::advance`]. Durations are in ms.
///
/// ```
/// use breaks::{BreaksPlanner, Event, Settings};
/// let settings = Settings { mini_break_notification: false, ..Settings::default() };
/// let mut planner = BreaksPlanner::new(settings, 0);
/// assert_eq!(planner.deadline(), Some(600_000));
/// assert_eq!(planner.advance(600_000), vec![Event::StartMiniBreak]);
/// ```
pub struct BreaksPlanner {
    settings: Settings,
    /// When the next Long break is due; its own clock, apart from Mini breaks.
    long_due: Option<i64>,
    postpones: u32,
    deadline: Option<i64>,
    delay: i64,
    paused: bool,
    dnd: bool,
    /// Idle for longer than the natural-break reset time (upstream `isSchedulerCleared`).
    away: bool,
    exclusion: bool,
    suspended: bool,
    next: Event,
    /// When the current wait for a break started.
    wait_start: i64,
    /// Start of the user's idle time while only active time counts.
    idle_since: Option<i64>,
    /// The wait is held from this time: the user is idle while only active time counts.
    held_since: Option<i64>,
}

impl BreaksPlanner {
    /// Pause duration that means "until resumed" (Stretchly's value).
    pub const INDEFINITELY: i64 = 1;

    /// Plans the first break from `now`.
    pub fn new(settings: Settings, now: i64) -> Self {
        let mut planner = Self {
            settings,
            long_due: None,
            postpones: 0,
            deadline: None,
            delay: 0,
            paused: false,
            dnd: false,
            away: false,
            exclusion: false,
            suspended: false,
            next: Event::StartMiniBreak,
            wait_start: now,
            idle_since: None,
            held_since: None,
        };
        planner.plan_next(now);
        planner
    }

    /// The settings the planner runs with.
    pub fn settings(&self) -> &Settings {
        &self.settings
    }

    /// When the pending step is due; `None` when nothing is scheduled (held, paused
    /// indefinitely, both break types off, or waiting for a manual finish).
    pub fn deadline(&self) -> Option<i64> {
        self.deadline.filter(|_| self.held_since.is_none())
    }

    /// Breaks are paused by the user or by suspend/lock.
    pub fn paused(&self) -> bool {
        self.paused
    }

    /// The user has been idle longer than the natural-break reset time.
    pub fn away(&self) -> bool {
        self.away
    }

    /// Do Not Disturb holds breaks.
    pub fn dnd(&self) -> bool {
        self.dnd
    }

    /// An app exclusion rule holds breaks.
    pub fn excluded(&self) -> bool {
        self.exclusion
    }

    /// The pending step (upstream's scheduler `reference`).
    pub fn next(&self) -> &Event {
        &self.next
    }

    /// Time until the next Long break is due on its own clock; `None` when Long breaks are
    /// off or no cycle runs.
    pub fn time_to_long_break(&self, now: i64) -> Option<i64> {
        let now = self.held_since.unwrap_or(now);
        self.settings
            .long_break
            .then_some(self.long_due?)
            .map(|due| (due - now).max(0))
    }

    /// A break is running: it has started and not yet finished.
    pub fn in_break(&self) -> bool {
        matches!(self.next, Event::FinishMiniBreak | Event::FinishLongBreak)
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
        if let Some((event, interval)) = notification.filter(|(_, interval)| self.delay > *interval)
        {
            self.deadline = self.deadline.map(|deadline| deadline - interval);
            self.delay -= interval;
            self.next = event;
        }
    }

    /// Moves the running break `*_postpone_time` later; returns whether it was allowed.
    ///
    /// Refused when postponing is off, the limit is used up, or too much of the break passed.
    pub fn postpone(&mut self, now: i64) -> bool {
        let Some(rule) = self.postpone_rule() else {
            return false;
        };
        if self.deadline.is_none()
            || self.postpones >= rule.limit
            || !can_postpone(rule.enabled, self.passed_percent(now), rule.percent)
        {
            return false;
        }
        self.postpones += 1;
        self.deadline = Some(now + rule.delay);
        self.delay = rule.delay;
        self.next = rule.start;
        if rule.delay > rule.notification_interval {
            self.add_notification();
        }
        true
    }

    /// Starts a Mini break after `delay` ms, or shortly when `None`. The Long break clock
    /// keeps running.
    pub fn skip_to_mini(&mut self, now: i64, delay: Option<i64>) {
        self.skip_to(Event::StartMiniBreak, now, delay);
    }

    /// Starts a Long break after `delay` ms, or shortly when `None`. The Long break clock
    /// restarts when it ends.
    pub fn skip_to_long(&mut self, now: i64, delay: Option<i64>) {
        self.skip_to(Event::StartLongBreak, now, delay);
    }

    fn skip_to(&mut self, start: Event, now: i64, delay: Option<i64>) {
        let delay = delay.unwrap_or(SKIP_DELAY_MS);
        // A break the user asks for comes even while they are idle.
        self.held_since = None;
        self.next = start;
        self.deadline = Some(now + delay);
        self.delay = delay;
    }

    /// Pauses breaks for `duration` ms, or until [`Self::resume`] with [`Self::INDEFINITELY`].
    pub fn pause(&mut self, now: i64, duration: i64) {
        self.paused = true;
        self.held_since = None;
        self.long_due = None;
        self.postpones = 0;
        self.next = Event::ResumeBreaks;
        self.deadline = if duration == Self::INDEFINITELY {
            None
        } else {
            Some(now + duration)
        };
        self.delay = duration;
    }

    /// Ends a pause and starts a new break cycle.
    ///
    /// Do Not Disturb, if on, still holds the breaks until it ends.
    pub fn resume(&mut self, now: i64) {
        // Upstream restarts its idle and exclusion polls here, and its DND poll finds DND
        // again a poll later; DND is pushed to us, so its state is kept instead.
        self.paused = false;
        self.suspended = false;
        self.away = false;
        self.exclusion = false;
        self.long_due = None;
        self.plan_next(now);
    }

    /// Starts a new break cycle (upstream `reset`), the same as [`Self::resume`].
    pub fn reset(&mut self, now: i64) {
        self.resume(now);
    }

    /// Time until the next break starts, including any warning before it; `None` when no
    /// break is scheduled or one is running.
    pub fn time_to_next_break(&self, now: i64) -> Option<i64> {
        let now = self.held_since.unwrap_or(now);
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

    /// How far the wait for the next break has come, 0–100 (for the tray icon).
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

    /// Ends the running break and plans the next one; returns the matching `Finish*` event,
    /// or `None` when no break is running.
    pub fn finish_break(&mut self, now: i64) -> Option<Event> {
        let event = self.in_break().then(|| self.next.clone())?;
        self.plan_next(now);
        Some(event)
    }

    // Upstream `nextBreak`, with Long breaks on their own clock (`longBreakEvery`). A Long
    // break due before the middle of the Mini break wait after the next one takes the next
    // Mini break's place and time, so two breaks never come close together. Both clocks restart after
    // a Long break.
    fn plan_next(&mut self, now: i64) {
        self.postpones = 0;
        self.held_since = None;
        self.wait_start = now;
        if self.next == Event::FinishLongBreak {
            self.long_due = None;
        }
        let s = &self.settings;
        let mini = s.mini_break.then(|| now + s.mini_break_interval);
        let long = s
            .long_break
            .then(|| *self.long_due.get_or_insert(now + s.long_break_every));
        let (at, start) = match (mini, long) {
            (None, None) => {
                self.deadline = None;
                return;
            }
            (Some(mini), Some(long)) if long <= mini + s.mini_break_interval / 2 => {
                (mini, Event::StartLongBreak)
            }
            (Some(mini), _) => (mini, Event::StartMiniBreak),
            (None, Some(long)) => (long, Event::StartLongBreak),
        };
        let at = at.max(now);
        self.deadline = Some(at);
        self.delay = at - now;
        self.next = start;
        self.add_notification();
        if self.dnd || self.exclusion || self.away {
            self.deadline = None;
        }
        self.hold_if_idle(now);
    }

    /// Takes the pending step if it is due at `now` and returns it; empty when nothing is due.
    pub fn advance(&mut self, now: i64) -> Vec<Event> {
        if !self.deadline().is_some_and(|deadline| now >= deadline) {
            return Vec::new();
        }
        let event = self.next.clone();
        let (delay, next) = match event {
            Event::FinishMiniBreak if self.settings.mini_break_manual_finish => {
                self.deadline = None;
                return vec![Event::AwaitMiniBreakFinish];
            }
            Event::FinishLongBreak if self.settings.long_break_manual_finish => {
                self.deadline = None;
                return vec![Event::AwaitLongBreakFinish];
            }
            Event::FinishMiniBreak | Event::FinishLongBreak => {
                self.plan_next(now);
                return vec![event];
            }
            Event::ResumeBreaks => {
                self.resume(now);
                return vec![event];
            }
            Event::MiniBreakNotification => (
                self.settings.mini_break_notification_interval,
                Event::StartMiniBreak,
            ),
            Event::LongBreakNotification => (
                self.settings.long_break_notification_interval,
                Event::StartLongBreak,
            ),
            Event::StartMiniBreak => (self.settings.mini_break_duration, Event::FinishMiniBreak),
            Event::StartLongBreak => (self.settings.long_break_duration, Event::FinishLongBreak),
            Event::AwaitMiniBreakFinish
            | Event::AwaitLongBreakFinish
            | Event::MiniBreakTakenNaturally
            | Event::LongBreakTakenNaturally => unreachable!("never a pending step"),
        };
        self.delay = delay;
        self.deadline = Some(now + delay);
        self.next = next;
        vec![event]
    }
}
